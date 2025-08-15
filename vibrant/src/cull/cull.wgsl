@group(0) @binding(0) var<storage, read_write> STATUS: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> OFFSET: atomic<u32>;
@group(0) @binding(2) var<uniform> START: u32;
@group(0) @binding(3) var<uniform> END: u32;
@group(0) @binding(4) var<storage, read_write> COUNT: atomic<u32>;

@group(1) @binding(0) var<storage, read_write> INDEX_IN: array<u32>;
@group(1) @binding(1) var<storage, read_write> INDEX_OUT: array<u32>;

const STATUS_NOPE: u32 = 0u;
const STATUS_LOCAL: u32 = 1u;
const STATUS_GLOBAL: u32 = 2u;

const WORKGROUP_SIZE: u32 = 1024;
const SUBGROUP_SIZE: u32 = 32;
const SUBGROUP_COUNT: u32 = 32;
const CHUNK_SIZE: u32 = 32;

var<workgroup> SUBGROUP_OFFSETS: array<u32, SUBGROUP_COUNT>;
var<workgroup> WORKGROUP_OFFSET: u32;

var<workgroup> WORKGROUP_INDEX: u32;

@compute
@workgroup_size(WORKGROUP_SIZE, 1, 1)
fn main(
    @builtin(local_invocation_index) local_index: u32,
    @builtin(subgroup_invocation_id) subgroup_index: u32
) {
    let subgroup_id = local_index / SUBGROUP_SIZE;

    loop {
        let workgroup_index = acquire_workgroup_index(local_index);
        let workgroup_offset = workgroup_index * CHUNK_SIZE * WORKGROUP_SIZE + START;
        let subgroup_offset = workgroup_offset + subgroup_id * SUBGROUP_SIZE * CHUNK_SIZE;

        if (workgroup_offset >= END) { break; }

        workgroupBarrier();

        // Clear Subgroup Offsets
        if (subgroup_index == 0) {
            SUBGROUP_OFFSETS[subgroup_id] = 0u;
        }

        if (local_index == 0) {
            WORKGROUP_OFFSET = 0u;
        }

        workgroupBarrier();

        // Grab Values
        var indices = array<u32, CHUNK_SIZE>();
        var indices_flags = 0u;

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = subgroup_offset + subgroup_index * CHUNK_SIZE + i;
            if (index >= END) { continue; }

            indices[i] = INDEX_IN[index];

            indices_flags |= (select(1u, 0u, cull(indices[i])) << i);
        }

        workgroupBarrier();

        // Subgroup Scan
        let thread_sum = countOneBits(indices_flags);
        let thread_offset = subgroupExclusiveAdd(thread_sum);

        if (subgroup_index == SUBGROUP_COUNT - 1u) {
            SUBGROUP_OFFSETS[subgroup_id] = thread_offset + thread_sum;
        }

        workgroupBarrier();

        // Workgroup Scan
        if (subgroup_id == 0) {
            let subgroup_count = SUBGROUP_OFFSETS[subgroup_index];
            let subgroup_offset = subgroupExclusiveAdd(subgroup_count);

            SUBGROUP_OFFSETS[subgroup_index] = subgroup_offset;

            if (subgroup_index == SUBGROUP_SIZE - 1) {
                atomicStore(&STATUS[workgroup_index], ((subgroup_offset + subgroup_count) << 2) | STATUS_LOCAL);
            }
        }

        workgroupBarrier();

        // Lookback
        if (local_index == 0) {
            let count = atomicLoad(&STATUS[workgroup_index]) >> 2;
            var offset = 0u;

            for (var i = i32(workgroup_index) - 1; i >= 0; i--) {
                let status = acquire_status(u32(i), local_index);

                offset += status.offset;

                if (status.flag == STATUS_GLOBAL || status.flag == STATUS_NOPE) {
                    break;
                }
            }

            atomicStore(&STATUS[workgroup_index], ((offset + count) << 2) | STATUS_GLOBAL);
            atomicMax(&COUNT, offset + count); // Compute Count After Culling

            WORKGROUP_OFFSET = offset;
        }

        workgroupBarrier();

        // Cull Indices
        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = subgroup_offset + subgroup_index * CHUNK_SIZE + i;
            let skip = ((indices_flags >> i) & 1u) == 0u;

            if (skip || index >= END) { continue; }

            let thread_offset_total = WORKGROUP_OFFSET + SUBGROUP_OFFSETS[subgroup_id] + thread_offset;
            let item_offset = countOneBits(indices_flags & ((1u << i) - 1u));

            INDEX_OUT[thread_offset_total + item_offset] = indices[i];
        }
    }
}

fn acquire_workgroup_index(local_index: u32) -> u32 {
    if (local_index == 0) {
        WORKGROUP_INDEX = atomicAdd(&OFFSET, 1u);
    }

    return workgroupUniformLoad(&WORKGROUP_INDEX);
}

struct Status {
    flag: u32,
    offset: u32
}

fn new_status(status: u32) -> Status {
    return Status(status & 3, status >> 2);
}

fn acquire_status(workgroup_index: u32, local_index: u32) -> Status {
    for (var safety = 0u; safety < 1024; safety++) {
        let status = new_status(atomicLoad(&STATUS[workgroup_index]));
        if (status.flag != STATUS_NOPE) { return status; }
    }

    return Status();
}
