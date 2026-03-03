fn gaussian(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2.0 * sigma * sigma));
}

@compute
@workgroup_size(4, 4, 1)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let size = textureDimensions(DESTINATION);
    if (pixel.x >= size.x || pixel.y >= size.y) { return; }

    let texel = 1.0 / vec2<f32>(size);
    let uv = (vec2<f32>(pixel.xy) + 0.5) * texel;
    let step = compute_step(uv, texel);

    let radius = i32(ENVIRONMENT.settings.blur_kernel_size);
    let sigma = f32(radius) / 3.0;

    var total_weight: f32 = 0.0;
    var result = vec3<f32>(0.0);

    for (var i: i32 = -radius; i <= radius; i++) {
        let w = gaussian(f32(i), sigma);
        result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv + step * f32(i), 0.0).rgb * w;
        total_weight += w;
    }

    result /= total_weight;

    textureStore(DESTINATION, pixel.xy, vec4<f32>(result, 1.0));
}
