@group(0) @binding(0) var<storage, read_write> HISTOGRAM: array<u32>;
@group(0) @binding(1) var<storage, read_write> STATUS: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> OFFSET: array<atomic<u32>>;
@group(0) @binding(3) var<storage> COUNT: u32;

@group(1) @binding(0) var<uniform> RADIX_SHIFT: u32;

@group(2) @binding(0) var<storage, read_write> KEYS_IN: array<u32>;
@group(2) @binding(1) var<storage, read_write> VALUES_IN: array<u32>;

@group(3) @binding(0) var<storage, read_write> KEYS_OUT: array<u32>;
@group(3) @binding(1) var<storage, read_write> VALUES_OUT: array<u32>;

const STATUS_NOPE: u32 = 0u;
const STATUS_LOCAL: u32 = 1u;
const STATUS_GLOBAL: u32 = 2u;

const WORKGROUP_SIZE: u32 = 1024;
const SUBGROUP_SIZE: u32 = 32;
const SUBGROUP_COUNT: u32 = 32;
const CHUNK_SIZE: u32 = 32;

var<workgroup> SUBGROUP_OFFSETS: array<atomic<u32>, RADIX * SUBGROUP_COUNT>;
var<workgroup> WORKGROUP_OFFSETS: array<atomic<u32>, RADIX>;

var<workgroup> WORKGROUP_OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE, 1, 1)
fn main(
    @builtin(local_invocation_index) local_index: u32,
    @builtin(subgroup_invocation_id) subgroup_index: u32
) {
    let subgroup_id = local_index / SUBGROUP_SIZE;

    loop {
        let workgroup_index = acquire_workgroup_index(local_index);
        let workgroup_offset = workgroup_index * CHUNK_SIZE * WORKGROUP_SIZE;
        let subgroup_offset = workgroup_offset + subgroup_id * SUBGROUP_SIZE * CHUNK_SIZE;

        if (workgroup_offset >= COUNT) { break; }

        workgroupBarrier();

        // Clear Statusses & Workgroup Offsets
        if (local_index < RADIX) {
            WORKGROUP_OFFSETS[local_index] = 0u;
            STATUS[workgroup_index * RADIX + local_index] = 0u;
        }

        // Clear Subgroup Offsets
        for (var i = 0u; i < 8u; i++) {
            atomicStore(&SUBGROUP_OFFSETS[WORKGROUP_SIZE * i + local_index], 0u);
        }

        workgroupBarrier();

        // Grab Values
        var values = array<u32, CHUNK_SIZE>();

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = subgroup_offset + i * CHUNK_SIZE + subgroup_index;
            if (index >= COUNT) { continue; }

            values[i] = VALUES_IN[index];
        }

        // Populate Subgroup Offsets for Data Chunk
        var offsets = array<u32, CHUNK_SIZE>();

        // (Warp/Subgroup)-Level Multi Split
        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let radix = extract_byte(values[i], RADIX_SHIFT);
            let subgroup_radix_index = subgroup_id * RADIX + radix;

            var wave_flags = U32_MAX;

            for (var k = 0u; k < 8; k++) {
                let t = bool((radix >> k) & 1u);
                wave_flags &= select(U32_MAX, 0u, t) ^ subgroupBallot(t).x;
            }

            let total_bits = countOneBits(wave_flags);
            let peer_bits = countOneBits(wave_flags & ((1u << subgroup_index) - 1u));
            let lowest_rank_peer = countTrailingZeros(wave_flags);

            var subgroup_exclusive_total = 0u;
            if (subgroup_index == lowest_rank_peer) {
                subgroup_exclusive_total = atomicAdd(&SUBGROUP_OFFSETS[subgroup_radix_index], total_bits);
            }

            offsets[i] = subgroupShuffle(subgroup_exclusive_total, lowest_rank_peer) + peer_bits;
        }

        workgroupBarrier();

        // (Block/Workgroup)-Wide Scan
        for (var i = 0u; i < 8u; i++) {
            let radix = subgroup_id * 8 + i;
            let subgroup_offset_index = subgroup_index * RADIX + radix;

            let count = atomicLoad(&SUBGROUP_OFFSETS[subgroup_offset_index]);
            let offset = subgroupExclusiveAdd(count);

            // Publish Workgroup Status
            if (subgroup_index == SUBGROUP_SIZE - 1) {
                atomicStore(&STATUS[workgroup_index * RADIX + radix], ((offset + count) << 2) | STATUS_LOCAL);
            }

            // Update Subgroup Offset
            atomicStore(&SUBGROUP_OFFSETS[subgroup_offset_index], offset);
        }

        workgroupBarrier();

        // Lookback
        if (local_index < RADIX) {
            let count = atomicLoad(&STATUS[workgroup_index * RADIX + local_index]) >> 2;
            var offset = 0u;

            for (var i = i32(workgroup_index) - 1; i >= 0; i--) {
                let status = acquire_radix_status(u32(i), local_index);

                offset += status.offset;

                if (status.flag == STATUS_GLOBAL || status.flag == STATUS_NOPE) {
                    break;
                }
            }

            atomicStore(&STATUS[workgroup_index * RADIX + local_index], ((offset + count) << 2) | STATUS_GLOBAL);
            atomicStore(&WORKGROUP_OFFSETS[local_index], offset);
        }

        workgroupBarrier();

        // Shuffle Keys and Values
        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = subgroup_offset + i * CHUNK_SIZE + subgroup_index;

            if (index >= COUNT) { continue; }

            let radix_index = extract_byte(values[i], RADIX_SHIFT);
            let subgroup_radix_index = subgroup_id * RADIX + radix_index;

            let global_radix_offset = HISTOGRAM[(RADIX_SHIFT >> 3) * RADIX + radix_index];
            let workgroup_offset = WORKGROUP_OFFSETS[radix_index];
            let subgroup_offset = SUBGROUP_OFFSETS[subgroup_radix_index];

            let offset = global_radix_offset + workgroup_offset + subgroup_offset + offsets[i];

            VALUES_OUT[offset] = values[i];
            KEYS_OUT[offset] = KEYS_IN[index];
        }
    }
}

fn acquire_workgroup_index(local_index: u32) -> u32 {
    if (local_index == 0) {
        WORKGROUP_OFFSET = atomicAdd(&OFFSET[1 + (RADIX_SHIFT >> 3)], 1u);
    }

    return workgroupUniformLoad(&WORKGROUP_OFFSET);
}

struct Status {
    flag: u32,
    offset: u32
}

fn new_status(status: u32) -> Status {
    return Status(status & 3, status >> 2);
}

fn acquire_radix_status(workgroup_index: u32, local_index: u32) -> Status {
    for (var safety = 0u; safety < 1024; safety++) {
        let status = new_status(atomicLoad(&STATUS[workgroup_index * RADIX + local_index]));
        if (status.flag != STATUS_NOPE) { return status; }
    }

    return Status();
}
