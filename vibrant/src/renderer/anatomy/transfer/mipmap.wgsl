@group(0) @binding(0) var SOURCE_ABSORPTION: texture_3d<f32>;
@group(0) @binding(1) var SOURCE_SCATTERING: texture_3d<f32>;
@group(0) @binding(2) var SOURCE_EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var SAMPLER: sampler;
@group(0) @binding(4) var DESTINATION_ABSORPTION: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(5) var DESTINATION_SCATTERING: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(6) var DESTINATION_EXTINCTION: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = vec3<f32>(textureDimensions(SOURCE_ABSORPTION));
    let sample = (2.0 * vec3<f32>(voxel) + 1.0) / dim;

    textureStore(DESTINATION_ABSORPTION, voxel,
        textureSampleLevel(SOURCE_ABSORPTION, SAMPLER, sample, 0.0));

    textureStore(DESTINATION_SCATTERING, voxel,
        textureSampleLevel(SOURCE_SCATTERING, SAMPLER, sample, 0.0));

    textureStore(DESTINATION_EXTINCTION, voxel,
        textureSampleLevel(SOURCE_EXTINCTION, SAMPLER, sample, 0.0));
}
