struct Vertex {
    @location(0) position: vec3<f32>
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> Fragment
{
    return Fragment(vec4<f32>(vertex.position / 100.0, 1.0));
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let dx = dpdx(fragment.position.xyz);
    let dy = dpdy(fragment.position.xyz);

    return vec4<f32>(dx, 1.0);
}