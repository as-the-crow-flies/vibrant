@compute
@workgroup_size(4, 4, 1)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {

}