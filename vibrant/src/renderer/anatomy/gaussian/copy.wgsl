@group(0) @binding(0) var SRC: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var DST: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(DST))) { return; }
    textureStore(DST, voxel, textureLoad(SRC, voxel));
}
