struct KBufferItem {
    depth: f32,
    color: u32
}

@group(0) @binding(0) var<storage, read_write> KBUFFER: array<KBufferItem>;
@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );
}

@fragment
fn fragment(@builtin(position) clip: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<u32>(clip.xy);
    let pixel_index = pixel.y * ENVIRONMENT.surface.x + pixel.x;

    var color = vec4<f32>();

    for (var k=0u; k < K; k++) {
        let index = pixel_index * K + K - k - 1u;
        let item = KBUFFER[index];

        color += (1.0 - color.a) * unpack4x8unorm(item.color);
    }

    return color;
}
