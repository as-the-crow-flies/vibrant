@group(0) @binding(3) var<storage, read_write> BIN: array<u32>;
@group(0) @binding(4) var<storage, read_write> THRESHOLD: f32;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(256)
fn main(
    @builtin(local_invocation_index) local: u32,
    @builtin(subgroup_invocation_id) subgroup: u32,
    @builtin(subgroup_size) subgroup_size: u32,
) {
    let count = BIN[local];
    let offset = workgroupExclusiveAdd(count, local, subgroup, subgroup_size);

    BIN[local] = offset + count;

    workgroupBarrier();

    let max_number = ENVIRONMENT.memory * 262144;

    let curr_overflow = BIN[local] > max_number;
    let last_overflow = BIN[select(local - 1u, 0u, local == 0u)] > max_number;
    let no_overflow = (local == 255) && !curr_overflow;

    if (no_overflow) {
        THRESHOLD = 2.0;
    }
    else if ((curr_overflow && !last_overflow)) {
        THRESHOLD = f32(local) * U8_MAX_INV;
    }
}
