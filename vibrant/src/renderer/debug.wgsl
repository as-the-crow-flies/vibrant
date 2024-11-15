@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(1) @binding(0) var SAMPLER: sampler;
@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment
{
    let clip = vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );

    let uv = vec2<f32>(
        select(0.0, 1.0, bool(index & 1)),
        select(1.0, 0.0, bool(index & 2))
    );

    return Fragment(clip, uv);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    return vec4<f32>(
        textureSampleLevel(
            SOURCE,
            SAMPLER,
            fragment.uv,
            ENVIRONMENT.settings.debug_level).xyz,
        1.0
    );
}
