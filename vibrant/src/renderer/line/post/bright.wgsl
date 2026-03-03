@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var COLOR: texture_2d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

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
    let color = textureSample(COLOR, SAMPLER, uv).rgb;

    let threshold = ENVIRONMENT.settings.bloom_threshold;
    let soft_knee = ENVIRONMENT.settings.bloom_soft_knee;

    // Convert RGB to perceived brightness (luminance).
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));

    // Soft knee avoids hard threshold edges and flickering near the cutoff.
    let knee = max(threshold * soft_knee, 1e-5);
    let soft = clamp((luminance - threshold + knee) / (2.0 * knee), 0.0, 1.0);
    let contribution = max(luminance - threshold, 0.0) + soft * soft * knee;

    // Normalize by luminance so hue is preserved when extracting highlights.
    let scale = contribution / max(luminance, 1e-5);

    return vec4<f32>(color * scale, 1.0);
}
