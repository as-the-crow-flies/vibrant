@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var DENSITY_SAMPLER: sampler;

@group(1) @binding(0) var OCCLUSION: texture_storage_3d<r32float, read_write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let position = vec3<f32>(voxel) + 0.5;

    var total = 0.0;

    if (density(position / vec3<f32>(textureDimensions(OCCLUSION)), 0.0) > 0.0) {
        total += 0.17 * occlusion(position, vec3<f32>( 0.0, 0.0, 1.0));
        total += 0.17 * occlusion(position, vec3<f32>( 0.0, 0.0,-1.0));
        total += 0.17 * occlusion(position, vec3<f32>( 0.0, 1.0, 0.0));
        total += 0.17 * occlusion(position, vec3<f32>( 0.0,-1.0, 0.0));
        total += 0.17 * occlusion(position, vec3<f32>( 1.0, 0.0, 0.0));
        total += 0.17 * occlusion(position, vec3<f32>(-1.0, 0.0, 0.0));
    }

    textureStore(OCCLUSION, voxel, vec4<f32>(total));
}

fn occlusion(position: vec3<f32>, direction: vec3<f32>) -> f32 {
    let zero = vec3<f32>(0.0);
    let dim = vec3<f32>(textureDimensions(OCCLUSION));
    let one_over_dim = 1.0 / dim;

    var occlusion = 0.0;

    for (var distance = 1.0; distance < dim.x; distance *= 2.0) {
        let sample = position + direction * distance;

        if (any(sample < zero) || any(sample >= dim)) { break; }

        occlusion += (1.0 - occlusion) * density(sample * one_over_dim, distance);
    }

    return occlusion / ENVIRONMENT.settings.alpha;
}

fn density(sample: vec3<f32>, level: f32) -> f32 {
    return textureSampleLevel(DENSITY, DENSITY_SAMPLER, sample, level).x;
}
