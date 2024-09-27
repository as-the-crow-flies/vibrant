@group(0) @binding(0) var<uniform> CAMERA: mat4x4<f32>;

struct Vertex {
    @location(0) position: vec3<f32>
}

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> Fragment
{
    let clip = CAMERA * vec4<f32>(vertex.position, 1.0);
    return Fragment(clip, vertex.position);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    return vec4<f32>(normalize(fwidth(fragment.position)), 1.0);
}
