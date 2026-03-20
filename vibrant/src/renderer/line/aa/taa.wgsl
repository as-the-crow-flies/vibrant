// TAA (Temporal Anti-Aliasing)

@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
// current frame color
@group(1) @binding(0) var CURRENT: texture_2d<f32>;
@group(1) @binding(1) var CURRENT_SAMPLER: sampler;
// history buffer
@group(2) @binding(0) var HISTORY: texture_2d<f32>;
@group(2) @binding(1) var HISTORY_SAMPLER: sampler;


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
    let texel = 1.0 / resolution;


    let uv = pixel.xy / resolution;

    let current = textureSample(CURRENT, CURRENT_SAMPLER, uv);

    let history = textureSample(HISTORY, HISTORY_SAMPLER, uv);



    // Variance clipping (3×3 neighborhood)
    // accumulate first and second moments
    var moment1 = vec4<f32>(0.0);   // Σ color
    var moment2 = vec4<f32>(0.0);   // Σ color²

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let offset = vec2<f32>(f32(dx), f32(dy)) * texel;
            let s = textureSample(CURRENT, CURRENT_SAMPLER, uv + offset);
            moment1 += s;
            moment2 += s * s;
        }
    }

    // mean and variance from the 9 neighborhood samples
    let mean = moment1 / 9.0;
    let variance = moment2 / 9.0 - mean * mean;
    let stddev = sqrt(max(variance, vec4<f32>(0.0)));

    // clamp the history sample
    let sigma = ENVIRONMENT.settings.taa_clamp_sigma;   // controls the clipping aggressiveness
    let clip_min = mean - sigma * stddev;
    let clip_max = mean + sigma * stddev;
    let history_clipped = clamp(history, clip_min, clip_max);



    // temporal blend
    let blend = ENVIRONMENT.settings.taa_blend_factor;

    return mix(history_clipped, current, blend);
}
