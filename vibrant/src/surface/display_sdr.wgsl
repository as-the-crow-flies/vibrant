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

fn tonemap_reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (1.0 + x);
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(UI));
    let uv = pixel.xy / dim;

    // Scene stays in linear HDR here, then gets mapped once for SDR output.
    let color = textureSample(COLOR, COLOR_SAMPLER, uv);
    let mapped = tonemap_reinhard(max(color.rgb, vec3<f32>(0.0)));

    // egui output is premultiplied-alpha; blend with "over" in linear space.
    let ui = textureSample(UI, UI_SAMPLER, uv);
    let alpha = clamp(ui.a, 0.0, 1.0);
    let composed = mapped * (1.0 - alpha) + ui.rgb;

    return vec4<f32>(composed, 1.0);
}
