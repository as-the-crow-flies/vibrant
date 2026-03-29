@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var PERIPHERAL: texture_2d<f32>;
@group(1) @binding(1) var PERIPHERAL_SAMPLER: sampler;
@group(2) @binding(0) var FOCUS: texture_2d<f32>;
@group(2) @binding(1) var FOCUS_SAMPLER: sampler;

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
    // Output is at normal resolution (render_width × render_height)
    let dim = vec2<f32>(ENVIRONMENT.surface);
    let uv = pixel.xy / dim;

    // Convert to NDC (-1, 1), flip Y to match screen coordinates
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, -(uv.y * 2.0 - 1.0));

    let focus_center = ENVIRONMENT.foveated_mouse;
    let dist = length(ndc - focus_center);
    let r = ENVIRONMENT.foveated_focus_radius;
    let bw = ENVIRONMENT.foveated_blend_width;

    // Smooth blend: 1.0 inside focus, 0.0 outside
    let blend = 1.0 - smoothstep(r - bw, r + bw, dist);

    // Sample peripheral (low-res, bilinear upscaled)
    let peripheral = textureSample(PERIPHERAL, PERIPHERAL_SAMPLER, uv);

    // Compute UV in the focus texture, flip Y for texture coordinates
    let focus_uv = (ndc - focus_center) * vec2<f32>(1.0, -1.0) / (2.0 * r) + 0.5;
    let in_bounds = all(focus_uv >= vec2<f32>(0.0)) && all(focus_uv <= vec2<f32>(1.0));

    if (in_bounds && blend > 0.001) {
        let focus = textureSample(FOCUS, FOCUS_SAMPLER, focus_uv);
        return mix(peripheral, focus, blend);
    }

    return peripheral;
}
