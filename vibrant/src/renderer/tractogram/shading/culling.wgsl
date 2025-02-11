@group(0) @binding(0) var OCCLUSION: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let uv = vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0));
    return Fragment(vec4<f32>(2.0 * uv - 1.0, 0.0, 1.0), uv);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let depth = textureSampleLevel(OCCLUSION, SAMPLER, fragment.uv, 0.0).x;
    return vec4<f32>(vec3<f32>(depth), 1.0);
}
