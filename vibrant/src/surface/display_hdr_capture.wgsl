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
    return peak * (vec3<f32>(1.0) - exp(-x / peak));
}

fn pq_encode_channel(nits: f32) -> f32 {
    let Lp: f32 = 10000.0;
    let m1: f32 = 0.1593017578125;
    let m2: f32 = 78.84375;
    let c1: f32 = 0.8359375;
    let c2: f32 = 18.8515625;
    let c3: f32 = 18.6875;
    let xp = pow(max(nits / Lp, 0.0), m1);
    return pow((c1 + c2 * xp) / (1.0 + c3 * xp), m2);
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0, 1.0,
    );
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(UI));
    let uv = pixel.xy / dim;

    let scene = textureSample(COLOR, COLOR_SAMPLER, uv);
    let ui = textureSample(UI, UI_SAMPLER, uv);
    let alpha = clamp(ui.a, 0.0, 1.0);

    let paper_white = clamp(HDR.paper_white_nits, 80.0, max(HDR.peak_nits, 80.0));
    let peak_nits_val = max(HDR.peak_nits, paper_white);
    let paper_white_scale = paper_white / 80.0;
    let peak_scale = peak_nits_val / 80.0;

    // Identical HDR mapping to display_hdr.wgsl
    let scene_hdr = soft_limit_peak(
        max(scene.rgb, vec3<f32>(0.0)) * paper_white_scale,
        peak_scale
    );
    let ui_hdr = min(ui.rgb * paper_white_scale, vec3<f32>(peak_scale));
    let composed = scene_hdr * (1.0 - alpha) + ui_hdr;

    // Convert scene-relative units to absolute nits
    // composed values are in units of (nits/80), so multiply by 80
    let nits = composed * 80.0;

    // Apply PQ transfer function — maps nits to 0-1
    let pq_r = pq_encode_channel(nits.r);
    let pq_g = pq_encode_channel(nits.g);
    let pq_b = pq_encode_channel(nits.b);

    // Output PQ encoded values in 0-1 range into Rgba16Float texture
    return vec4<f32>(pq_r, pq_g, pq_b, 1.0);
}