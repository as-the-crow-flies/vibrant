// SMAA Pass 3: Neighborhood blending.

// Uses the blend weights computed in Pass 2 to sample and mix neighboring pixels,
// producing the final anti-aliased image

@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var COLOR: texture_2d<f32>;
@group(1) @binding(1) var COLOR_SAMPLER: sampler;
@group(2) @binding(0) var BLEND: texture_2d<f32>;
@group(2) @binding(1) var BLEND_SAMPLER: sampler;

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
    let resolution = vec2<f32>(ENVIRONMENT.surface);
    let uv = pixel.xy / resolution;
    let texel = 1.0 / resolution;

    let weights = textureSample(BLEND, BLEND_SAMPLER, uv);

    let total_weight = dot(weights, vec4<f32>(1.0));

    if (total_weight < 1e-5) {
        // no blending needed
        return textureSample(COLOR, COLOR_SAMPLER, uv);
    }

    // Blend the current pixel with its neighbors with SMAA blend weights.
    var blended = vec4<f32>(0.0);

    blended += weights.r * textureSample(COLOR, COLOR_SAMPLER, uv + vec2<f32>(0.0, -texel.y));
    blended += weights.g * textureSample(COLOR, COLOR_SAMPLER, uv + vec2<f32>(0.0,  texel.y));
    blended += weights.b * textureSample(COLOR, COLOR_SAMPLER, uv + vec2<f32>(-texel.x, 0.0));
    blended += weights.a * textureSample(COLOR, COLOR_SAMPLER, uv + vec2<f32>( texel.x, 0.0));

    let blend_factor = min(total_weight, 1.0);
    let original = textureSample(COLOR, COLOR_SAMPLER, uv);

    return mix(original, blended / total_weight, blend_factor);
}
