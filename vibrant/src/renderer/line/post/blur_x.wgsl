@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var COLOR: texture_2d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

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
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pixel.xy / vec2<f32>(ENVIRONMENT.surface);

    // Horizontal pass of separable Gaussian blur.
    let texel = vec2<f32>(1.0 / f32(ENVIRONMENT.surface.x), 0.0);

    var sum = textureSample(COLOR, SAMPLER, uv).rgb * 0.227027;
    sum += textureSample(COLOR, SAMPLER, uv + texel * 1.384615).rgb * 0.316216;
    sum += textureSample(COLOR, SAMPLER, uv - texel * 1.384615).rgb * 0.316216;
    sum += textureSample(COLOR, SAMPLER, uv + texel * 3.230769).rgb * 0.070270;
    sum += textureSample(COLOR, SAMPLER, uv - texel * 3.230769).rgb * 0.070270;

    return vec4<f32>(sum, 1.0);
}
