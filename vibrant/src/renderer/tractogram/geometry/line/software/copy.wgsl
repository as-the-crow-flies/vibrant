@group(0) @binding(0) var<storage> KBUFFER: array<array<u64, SURFACE_X>, SURFACE_Y>;
@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

const U32_MAX: u32 = 4294967295;

struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
}

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let ndc = 2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0;
    return Fragment(vec4<f32>(ndc, 0.0, 1.0), ndc);
}

@fragment
fn fragment(fragment: Fragment) -> GBuffer {
    let pixel = vec2<u32>(fragment.clip.xy);

    let visibility = KBUFFER[pixel.y][pixel.x];

    let payload = unpack4x8unorm(u32(visibility));
    let depth = f32(u32(visibility >> 32u)) / f32(U32_MAX);

    if (depth == 1.0) { return GBuffer(); }

    let ndc = vec4<f32>(fragment.ndc, depth, 1.0/depth);
    let position = ENVIRONMENT.camera.projection_inverse * ndc;

    return GBuffer(
        vec4<f32>(position.xyz / position.w, 1.0),
        vec4<f32>(vec3<f32>(10.0 * payload.w), 1.0),
        vec4<f32>(payload.xyz, 1.0)
    );
}
