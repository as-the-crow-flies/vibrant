@group(0) @binding(0) var COLOR: texture_2d<f32>;
@group(0) @binding(1) var COLOR_SAMPLER: sampler;
@group(1) @binding(0) var UI: texture_2d<f32>;
@group(1) @binding(1) var UI_SAMPLER: sampler;

struct HdrParams {
    paper_white_nits: f32,
    peak_nits: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(2) @binding(0) var<uniform> HDR: HdrParams;

fn soft_limit_peak(x: vec3<f32>, peak: f32) -> vec3<f32> {
    // Smoothly approaches the peak instead of hard clipping.
    return peak * (vec3<f32>(1.0) - exp(-x / peak));
}

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
    let dim = vec2<f32>(textureDimensions(COLOR));
    let uv = pixel.xy / dim;

    // Sample scene in linear working space.
    let scene = textureSample(COLOR, COLOR_SAMPLER, uv);

    // egui output is premultiplied-alpha; blend with "over" in linear space.
    let ui = textureSample(UI, UI_SAMPLER, uv);
    let alpha = clamp(ui.a, 0.0, 1.0);

    // Build absolute luminance mapping from SDR reference white (~80 nits).
    let paper_white = clamp(HDR.paper_white_nits, 80.0, max(HDR.peak_nits, 80.0));
    let peak_nits = max(HDR.peak_nits, paper_white);
    let paper_white_scale = paper_white / 80.0;
    let peak_scale = peak_nits / 80.0;

    // Map scene value 1.0 to paper-white and apply peak-dependent soft roll-off.
    let scene_hdr = soft_limit_peak(max(scene.rgb, vec3<f32>(0.0)) * paper_white_scale, peak_scale);

    // UI remains SDR-authored content; place it at paper-white and cap by peak.
    let ui_hdr = min(ui.rgb * paper_white_scale, vec3<f32>(peak_scale));
    let composed = scene_hdr * (1.0 - alpha) + ui_hdr;

    return vec4<f32>(composed, 1.0);
}
