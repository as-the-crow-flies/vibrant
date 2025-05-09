@group(0) @binding(0) var<storage> COLOR: array<u32>;
@group(0) @binding(3) var ABSORBANCE: texture_storage_2d<r8unorm, read>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) clip: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<u32>(clip.xy);

    let dim = vec3<u32>(ENVIRONMENT.surface, ENVIRONMENT.layers);

    let tile = vec2<u32>(pixel.x, div_ceil(dim.y, ENVIRONMENT.tile) * ENVIRONMENT.tile - pixel.y) / ENVIRONMENT.tile;

    let absorbance = precision_decode(textureLoad(ABSORBANCE, tile).x) * ENVIRONMENT.settings.culling_threshold;
    let absorbance_per_layer = absorbance / f32(ENVIRONMENT.layers);

    var result = vec4<f32>();

    for (var layer=0u; layer<ENVIRONMENT.layers; layer++) {
        let offset = block_index(vec3<u32>(pixel, layer), dim);

        let color = rgba_decode(
            vec2<u32>(COLOR[2 * offset + 0], COLOR[2 * offset + 1]),
            absorbance_per_layer
        );

        result += (1.0 - result.a) * color / max(color.a, 1.0);
    }

    return result;
}
