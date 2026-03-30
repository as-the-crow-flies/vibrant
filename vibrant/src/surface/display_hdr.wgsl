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

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0, 1.0,
    );
}

<<<<<<< Updated upstream
// map scene linear values to absolute nits for HDR displayip.
fn hdr_grade(x: vec3<f32>, paper: f32, peak: f32) -> vec3<f32> {
    // linear portion: scene [0, 1] maps exactly to [0, paper] nits
    let linear = x * paper;
    // excess above SDR white (in scene-linear space)
    let excess = max(x - vec3<f32>(1.0), vec3<f32>(0.0));
    // available headroom above paper white in output space
    let headroom = peak - paper;
    let compressed = headroom * (vec3<f32>(1.0) - exp(-excess * paper / max(headroom, paper * 1e-4)));
    return linear + compressed;
=======
fn soft_limit_peak(x: vec3<f32>, peak: f32) -> vec3<f32> {
    return peak * (vec3<f32>(1.0) - exp(-x / peak));
>>>>>>> Stashed changes
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(UI));
    let uv = pixel.xy / dim;

    let scene = textureSample(COLOR, COLOR_SAMPLER, uv);
    let ui = textureSample(UI, UI_SAMPLER, uv);
    let alpha = clamp(ui.a, 0.0, 1.0);

<<<<<<< Updated upstream
    // Resolve nit targets (80 nit reference white per scRGB spec).
=======
>>>>>>> Stashed changes
    let paper_white = clamp(HDR.paper_white_nits, 80.0, max(HDR.peak_nits, 80.0));
    let peak_nits   = max(HDR.peak_nits, paper_white);
    // scRGB: 1.0 unit = 80 nits.
    let paper = paper_white / 80.0;
    let peak  = peak_nits  / 80.0;

<<<<<<< Updated upstream
    // Scene: linear below paper white, highlight roll-off above it.
    let scene_hdr = hdr_grade(max(scene.rgb, vec3<f32>(0.0)), paper, peak);

    // UI: SDR-authored, place at paper-white and cap at peak.
    let ui_hdr = min(ui.rgb * paper, vec3<f32>(peak));
=======
    // Output raw HDR values above 1.0
    let scene_hdr = soft_limit_peak(
        max(scene.rgb, vec3<f32>(0.0)) * paper_white_scale,
        peak_scale
    );
    let ui_hdr = min(ui.rgb * paper_white_scale, vec3<f32>(peak_scale));
>>>>>>> Stashed changes
    let composed = scene_hdr * (1.0 - alpha) + ui_hdr;

    // No normalization — output raw HDR values
    // These will be above 1.0 for bright areas
    return vec4<f32>(composed, 1.0);
}