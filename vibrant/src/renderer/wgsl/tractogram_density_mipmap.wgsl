@group(0) @binding(0) var SRC: texture_3d<f32>;
@group(0) @binding(1) var DST: texture_storage_3d<r32float, write>;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= vec3<u32>(VOLUME_XYZ))) { return; }

}
