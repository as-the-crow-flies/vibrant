@group(0) @binding(0) var ABSORPTION: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(1) var SCATTERING: texture_storage_3d<rgba8unorm, read_write>;
@group(0) @binding(2) var EXTINCTION: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(ABSORPTION))) { return; }

    textureStore(ABSORPTION, voxel, vec4<f32>(0.0));
    textureStore(SCATTERING, voxel, vec4<f32>(0.0));
    textureStore(EXTINCTION, voxel, vec4<f32>(0.0));
}
