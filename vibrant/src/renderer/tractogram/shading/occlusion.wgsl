@group(0) @binding(0) var OCCLUSION: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
    let surface_dim = vec2<f32>(ENVIRONMENT.surface);
    let occlusion_dim = vec3<f32>(textureDimensions(OCCLUSION, 0));

    let sample = vec3<f32>(fragment.uv, 1.0);
    let occlusion = textureSampleLevel(OCCLUSION, SAMPLER, sample, 0.0).x;

    return vec4<f32>(vec3<f32>(occlusion), 1.0);
}
