@group(0) @binding(1) var<storage, read_write> COUNT: u32;
@group(0) @binding(2) var<storage, read_write> WORKGROUP_COUNT: u32;

@compute
@workgroup_size(1, 1, 1)
fn compute() {
    WORKGROUP_COUNT = (COUNT + WORKGROUP_X - 1) / WORKGROUP_X;
}
