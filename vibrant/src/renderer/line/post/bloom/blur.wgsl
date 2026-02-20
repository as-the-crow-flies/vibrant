@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var SOURCE: texture_2d<f32>;
@group(1) @binding(1) var SOURCE_SAMPLER: sampler;
@group(2) @binding(0) var DESTINATION: texture_storage_2d<rgba16float, write>;

fn compute_step(uv: vec2<f32>, texel: vec2<f32>) -> vec2<f32> {
    return DIRECTION * texel * ENVIRONMENT.settings.bloom_spread;
}
