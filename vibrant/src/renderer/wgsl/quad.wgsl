@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32>
{
    return vec4<f32>(2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0, 0.0, 1.0);
}
