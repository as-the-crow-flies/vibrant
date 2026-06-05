@group(0) @binding( 0) var IRRADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding( 1) var RADIANCE_0: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    textureStore(IRRADIANCE, voxel, pack_rgb(vec3<f32>(1.0)));
}
