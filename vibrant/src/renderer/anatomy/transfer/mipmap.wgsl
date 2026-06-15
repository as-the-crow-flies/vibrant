@group(0) @binding(0) var ABSORPTION_SRC: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING_SRC: texture_3d<f32>;
@group(0) @binding(2) var EXTINCTION_SRC: texture_3d<f32>;
@group(0) @binding(3) var ABSORPTION_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var SCATTERING_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(5) var EXTINCTION_DST: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(6) var SAMPLER: sampler;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let sample = (2.0 * vec3<f32>(voxel) + 1.0) / vec3<f32>(textureDimensions(ABSORPTION_SRC));

    textureStore(ABSORPTION_DST, voxel, vec4<f32>(textureSampleLevel(ABSORPTION_SRC, SAMPLER, sample, 0.0)));
    textureStore(SCATTERING_DST, voxel, vec4<f32>(textureSampleLevel(SCATTERING_SRC, SAMPLER, sample, 0.0)));
    textureStore(EXTINCTION_DST, voxel, vec4<f32>(textureSampleLevel(EXTINCTION_SRC, SAMPLER, sample, 0.0)));
}
