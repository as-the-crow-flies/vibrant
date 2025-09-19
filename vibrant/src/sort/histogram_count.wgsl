@group(0) @binding(0) var<storage, read_write> HISTOGRAM: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> OFFSET: array<atomic<u32>>;
@group(0) @binding(3) var<storage> COUNT: u32;

@group(1) @binding(0) var<storage, read_write> KEYS: array<u32>;

var<workgroup> HISTOGRAM_WORKGROUP: array<atomic<u32>, 1024>;

var<workgroup> WORKGROUP_OFFSET: u32;

const WORKGROUP_SIZE: u32 = 1024;
const SUBGROUP_SIZE: u32 = 32;
const CHUNK_SIZE: u32 = 32;

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

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = subgroup_offset + i * CHUNK_SIZE + subgroup_index;
            if (index >= COUNT) { continue; }

            let key = KEYS[index];

            atomicAdd(&HISTOGRAM_WORKGROUP[extract_byte(key,  0) + 0 * RADIX], 1u);
            atomicAdd(&HISTOGRAM_WORKGROUP[extract_byte(key,  8) + 1 * RADIX], 1u);
            atomicAdd(&HISTOGRAM_WORKGROUP[extract_byte(key, 16) + 2 * RADIX], 1u);
            atomicAdd(&HISTOGRAM_WORKGROUP[extract_byte(key, 24) + 3 * RADIX], 1u);
        }
    }

    workgroupBarrier();

    atomicAdd(&HISTOGRAM[local_index], atomicLoad(&HISTOGRAM_WORKGROUP[local_index]));
}

fn acquire_workgroup_index(local_index: u32) -> u32 {
    if (local_index == 0) {
        WORKGROUP_OFFSET = atomicAdd(&OFFSET[0], 1u);
    }

    return workgroupUniformLoad(&WORKGROUP_OFFSET);
}
