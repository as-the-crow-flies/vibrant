@group(0) @binding(0) var SOURCE_ABSORPTION_TRANSMISSION: texture_3d<f32>;
@group(0) @binding(1) var SOURCE_SCATTERING_ROUGHNESS: texture_3d<f32>;
@group(0) @binding(2) var SAMPLER: sampler;
@group(0) @binding(3) var DESTINATION_ABSORPTION_TRANSMISSION: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var DESTINATION_SCATTERING_ROUGHNESS: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = vec3<f32>(textureDimensions(SOURCE_ABSORPTION_TRANSMISSION));
    let sample = (2.0 * vec3<f32>(voxel) + 1.0) / dim;

    textureStore(DESTINATION_ABSORPTION_TRANSMISSION, voxel,
        textureSampleLevel(SOURCE_ABSORPTION_TRANSMISSION, SAMPLER, sample, 0.0));

    textureStore(DESTINATION_SCATTERING_ROUGHNESS, voxel,
        textureSampleLevel(SOURCE_SCATTERING_ROUGHNESS, SAMPLER, sample, 0.0));
}
