@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var SCENE: texture_2d<f32>;
@group(1) @binding(1) var SCENE_SAMPLER: sampler;
@group(2) @binding(0) var BLOOM: texture_2d<f32>;
@group(2) @binding(1) var BLOOM_SAMPLER: sampler;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pixel.xy / vec2<f32>(ENVIRONMENT.surface);

    let intensity = ENVIRONMENT.settings.bloom_intensity;

    let scene = textureSample(SCENE, SCENE_SAMPLER, uv).rgb;
    let bloom = textureSample(BLOOM, BLOOM_SAMPLER, uv).rgb;

    // Add bloom contribution back to the original scene color.
    let combined = scene + bloom * intensity;

    return vec4<f32>(combined, 1.0);
}
