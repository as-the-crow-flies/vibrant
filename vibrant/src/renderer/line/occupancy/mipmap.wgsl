@group(0) @binding(0) var SOURCE: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(2) var DESTINATION: texture_storage_3d<r32float, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let sample = (2.0 * vec3<f32>(voxel) + 1.0) / vec3<f32>(textureDimensions(SOURCE));
    textureStore(DESTINATION, voxel, textureSampleLevel(SOURCE, SAMPLER, sample, 0.0));
}
