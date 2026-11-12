// Final present pass: read the linear HDR scene, tone map it once, and
// composite the (premultiplied, sRGB-encoded) egui overlay on top.
//
// Overridden per pipeline at creation (see `PresentPipeline`):
//   HDR            - drive highlights past SDR white toward `PRESENT.headroom`
//                    instead of landing everything in [0, 1].
//   LINEAR_OUTPUT  - the swapchain wants linear values (scRGB `ExtendedSrgbLinear`,
//                    native HDR). Otherwise it wants sRGB-encoded values -
//                    `Bgra8Unorm`/`Auto` (SDR) or `ExtendedSrgb` (web HDR, an
//                    sRGB curve extended past 1.0).
//   EXPORT         - screenshot variant: keep the scene alpha for a transparent
//                    background instead of the opaque 1.0 the swapchain needs.

use color;

@group(0) @binding(0) var SCENE: texture_2d<f32>;
@group(1) @binding(0) var OVERLAY: texture_2d<f32>;

struct Present {
    headroom: f32,
    exposure: f32,
    // 1 / samples in the scene.
    blur: f32,
};
@group(2) @binding(0) var<uniform> PRESENT: Present;

const HDR: bool = false;
const LINEAR_OUTPUT: bool = false;
const EXPORT: bool = false;

fn tonemap(color: vec3<f32>, peak: f32) -> vec3<f32> {
    let c = aces(color);
    let m = max(c.r, max(c.g, c.b));
    let ramp = smoothstep(0.0, 1.0, m);
    let lifted = m + (peak - 1.0) * ramp * ramp;
    return min(c * (lifted / max(m, 1e-4)), vec3<f32>(peak));
}

// A single noisy sample reads darker once tone mapped: the curve compresses
// its bright outliers more than its dark ones. Blurring the first samples a
// little keeps the brightness steady as they accumulate; it fades as 1/N.
fn scene_at(coord: vec2<u32>) -> vec4<f32> {
    let center = vec2<i32>(coord);
    let last = vec2<i32>(textureDimensions(SCENE)) - 1;

    var blurred = vec4<f32>(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            // 3×3 binomial: ¼, ½, ¼ per axis.
            let weight = (0.5 - 0.25 * f32(abs(x))) * (0.5 - 0.25 * f32(abs(y)));
            blurred += weight * textureLoad(SCENE, clamp(center + vec2<i32>(x, y), vec2<i32>(0), last), 0);
        }
    }

    return mix(textureLoad(SCENE, center, 0), blurred, PRESENT.blur);
}

// -----------------------------------------------------------------------

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
    let coord = vec2<u32>(pixel.xy);

    let texel = scene_at(coord);
    let scene = texel.rgb * PRESENT.exposure;

    let tone = tonemap(scene, select(1.0, PRESENT.headroom, HDR));
    let mapped = select(linear_to_srgb(tone), tone, LINEAR_OUTPUT);

    let overlay = textureLoad(OVERLAY, coord, 0); // premultiplied alpha, gamma space
    var ui = overlay.rgb;
    if (LINEAR_OUTPUT) {
        // Un-premultiply, linearize, re-premultiply so the UI sits at SDR white.
        let straight = select(overlay.rgb / max(overlay.a, 1e-5), vec3<f32>(0.0), overlay.a <= 0.0);
        ui = srgb_to_linear(straight) * overlay.a;
    }

    let alpha = select(1.0, overlay.a + texel.a * (1.0 - overlay.a), EXPORT);

    return vec4<f32>(mapped * (1.0 - overlay.a) + ui, alpha);
}
