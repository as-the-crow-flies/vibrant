@group(0) @binding(0) var<storage, read_write> HISTOGRAM: array<u32>;

var<workgroup> SUBGROUP_SUM: array<u32, 8>;

@compute
@workgroup_size(256, 1, 1)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
    @builtin(subgroup_invocation_id) subgroup_index: u32,
    @builtin(subgroup_size) subgroup_size: u32
) {
    let subgroup_id = local_index / subgroup_size;

    let index = workgroup_id.x * RADIX + local_index;
    let histogram = HISTOGRAM[index];

    let histogram_scan_subgroup = subgroupExclusiveAdd(histogram);

    if (subgroup_index == subgroup_size - 1) {
        SUBGROUP_SUM[subgroup_id] = histogram_scan_subgroup + histogram;
    }

    workgroupBarrier();

    if (local_index < 8) {
        SUBGROUP_SUM[subgroup_index] = subgroupExclusiveAdd(SUBGROUP_SUM[subgroup_index]);
    }

    workgroupBarrier();

    HISTOGRAM[index] = SUBGROUP_SUM[subgroup_id] + histogram_scan_subgroup;
}
