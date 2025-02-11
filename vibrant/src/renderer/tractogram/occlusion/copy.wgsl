@group(0) @binding(0) var OCCLUSION: texture_storage_3d<r32float, read_write>;
@group(0) @binding(3) var<uniform> OCCLUSION_TO_PROJECTION: mat4x4<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var DENSITY_SAMPLER: sampler;
@group(1) @binding(2) var<uniform> WORLD_TO_DENSITY: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) froxel: vec3<u32>) {
    if (any(froxel > textureDimensions(OCCLUSION))) { return; }

    let sample = transform(froxel);
    let height = 0.0;

    let density = textureSampleLevel(DENSITY, DENSITY_SAMPLER, sample, max(log2(height), 0.0));
    textureStore(OCCLUSION, froxel, density);
}

fn transform(froxel: vec3<u32>) -> vec3<f32> {
    let transform = ENVIRONMENT.camera.projection_inverse * OCCLUSION_TO_PROJECTION;
    let sample = transform * vec4<f32>(vec3<f32>(froxel), 1.0);
    return sample.xyz / sample.w + 0.5;
}
