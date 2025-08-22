@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var CULLING: texture_3d<f32>;

@group(2) @binding(0) var DIRECTIONAL: texture_storage_3d<r32float, read_write>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = f32(textureDimensions(DIRECTIONAL).x);
    let one_over_dim = 1.0 / dim;
    let one_over_alpha = 1.0 / ENVIRONMENT.settings.alpha;

    let position = vec3<f32>(voxel) + 0.5;
    let direction = ENVIRONMENT.light;

    var occlusion = 0.0;

    if (culling(position * one_over_dim, 1.0) > 0.0) {
        for (var distance = 1.0; distance < dim; distance += 1.0) {
            let sample = (position + direction * distance) * one_over_dim;

            occlusion += (1.0 - occlusion) * density(sample, 0.0);

            if (occlusion > 0.99 || any(abs(sample - 0.5) >= vec3<f32>(0.5))) { break; }
        }

        textureStore(DIRECTIONAL, voxel, vec4<f32>(occlusion));
    }
}

fn density(sample: vec3<f32>, level: f32) -> f32 {
    return textureSampleLevel(DENSITY, SAMPLER, sample, level).x;
}

fn culling(sample: vec3<f32>, level: f32) -> f32 {
    return textureSampleLevel(CULLING, SAMPLER, sample, level).x;
}
