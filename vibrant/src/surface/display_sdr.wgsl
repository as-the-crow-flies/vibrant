@group(0) @binding(0) var COLOR: texture_2d<f32>;
@group(0) @binding(1) var COLOR_SAMPLER: sampler;
@group(1) @binding(0) var UI: texture_2d<f32>;
@group(1) @binding(1) var UI_SAMPLER: sampler;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0,
    );
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(UI));
    let uv = pixel.xy / dim;

    let color = textureSample(COLOR, COLOR_SAMPLER, uv);

    // egui output is premultiplied-alpha; blend with "over" in linear space.
    let ui = textureSample(UI, UI_SAMPLER, uv);
    let alpha = clamp(ui.a, 0.0, 1.0);
    let composed = color.rgb * (1.0 - alpha) + ui.rgb;

    return vec4<f32>(composed, 1.0);
}
