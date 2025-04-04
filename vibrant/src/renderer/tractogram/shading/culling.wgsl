@group(0) @binding(0) var HIZ: texture_2d<f32>;
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
    let level = min(u32(ENVIRONMENT.settings.level), textureNumLevels(HIZ) - 1u);
    let surface_dim = vec2<f32>(ENVIRONMENT.surface);
    let hiz_dim = vec2<f32>(textureDimensions(HIZ, i32(level)) * (ENVIRONMENT.tile << level));

    let sample = surface_dim / hiz_dim * fragment.uv;
    let depth = textureSampleLevel(HIZ, SAMPLER, sample, f32(level)).x;

    return vec4<f32>(vec3<f32>(depth), 1.0);
}
