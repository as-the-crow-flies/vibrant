@group(0) @binding(1) var<storage, read_write> COUNT: u32;
@group(0) @binding(2) var<storage, read_write> WORKGROUP_COUNT: u32;
@group(0) @binding(3) var<storage, read_write> WORKGROUP_COUNT_2: u32;

@compute
@workgroup_size(1, 1, 1)
fn compute() {
    let WORKGROUP_X = 256u;
    let WORKGROUP_X_2 = 512u;
    WORKGROUP_COUNT = (COUNT + WORKGROUP_X - 1) / WORKGROUP_X;
    WORKGROUP_COUNT_2 = (COUNT + WORKGROUP_X_2 - 1) / WORKGROUP_X_2;
}
