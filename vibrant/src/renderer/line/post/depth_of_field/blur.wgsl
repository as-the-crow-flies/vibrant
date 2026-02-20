@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var SOURCE: texture_2d<f32>;
@group(1) @binding(1) var SOURCE_SAMPLER: sampler;
@group(2) @binding(0) var DESTINATION: texture_storage_2d<rgba16float, write>;
@group(3) @binding(0) var DEPTH: texture_2d<f32>;
@group(3) @binding(1) var DEPTH_SAMPLER: sampler;

fn compute_step(uv: vec2<f32>, texel: vec2<f32>) -> vec2<f32> {
    let depth = textureSampleLevel(DEPTH, DEPTH_SAMPLER, uv, 0.0).r;
    let coc = abs(depth - ENVIRONMENT.settings.focal_distance) * ENVIRONMENT.settings.aperture;
    return DIRECTION * texel * coc;
}
