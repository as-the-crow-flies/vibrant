struct Quad {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Quad
{
    let uv = vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0));

    return Quad(vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0), uv);
}

@fragment
fn fragment(quad: Quad) -> @location(0) vec4<f32> {
    return vec4<f32>(vec3<f32>(quad.uv, 1.0) * 0.5, 1.0);
}
