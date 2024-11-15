@group(0) @binding(0) var SOURCE: texture_2d<f32>;

struct Fragment {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Fragment
{
    let quad = vec2<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2))
    );

    return Fragment(vec4<f32>(quad, 0.0, 1.0));
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let index = vec2<u32>(fragment.position.xy) * 2u;

    let depth = max(max(max(
        textureLoad(SOURCE, index                  , 0).x,
        textureLoad(SOURCE, index + vec2<u32>(0, 1), 0).x),
        textureLoad(SOURCE, index + vec2<u32>(1, 0), 0).x),
        textureLoad(SOURCE, index + vec2<u32>(1, 1), 0).x);

    return vec4<f32>(vec3<f32>(depth), 1.0);
}
