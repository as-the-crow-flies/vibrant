@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var SOURCE: texture_2d<f32>;
@group(1) @binding(1) var SOURCE_SAMPLER: sampler;
@group(2) @binding(0) var DESTINATION: texture_storage_2d<rgba16float, write>;
@group(3) @binding(0) var DEPTH: texture_2d<f32>;
@group(3) @binding(1) var DEPTH_SAMPLER: sampler;

const SIGMA: f32 = 2.5;  // focal length

// fn gaussianWeight(x: f32, sigma: f32) -> f32 {
//     return 1 / (sqrt(2 * PI * sigma^2) * exp(-x^2 / (2 * sigma^2)));
// }
// G(x) = \frac{1}{\sqrt{2\pi\sigma^2}}e^{-\frac{x^2}{2\sigma^2}}
// one-dimensional Gaussian kernel
// fig 5.17 : https://nana.lecturer.pens.ac.id/index_files/referensi/computer_vision/Computer%20Vision.pdf
const W_1 = 1 / (sqrt(2 * PI * SIGMA * SIGMA) * exp(-(1 * 1) / (2 * SIGMA * SIGMA)));  // gaussianWeight(1.0, SIGMA);
const W_2 = 1 / (sqrt(2 * PI * SIGMA * SIGMA) * exp(-(2 * 2) / (2 * SIGMA * SIGMA)));  // gaussianWeight(2.0, SIGMA);
const W_3 = 1 / (sqrt(2 * PI * SIGMA * SIGMA) * exp(-(3 * 3) / (2 * SIGMA * SIGMA)));  // gaussianWeight(3.0, SIGMA);
const W_4 = 1 / (sqrt(2 * PI * SIGMA * SIGMA) * exp(-(4 * 4) / (2 * SIGMA * SIGMA)));  // gaussianWeight(4.0, SIGMA);
const W_5 = 1 / (sqrt(2 * PI * SIGMA * SIGMA) * exp(-(5 * 5) / (2 * SIGMA * SIGMA)));  // gaussianWeight(5.0, SIGMA);

const W_NORMALIZED = 2 + W_1 + W_2 + W_3 + W_4 + W_5;
const N_1 = W_1 / W_NORMALIZED;
const N_2 = W_2 / W_NORMALIZED;
const N_3 = W_3 / W_NORMALIZED;
const N_4 = W_4 / W_NORMALIZED;
const N_5 = W_5 / W_NORMALIZED;

@compute
@workgroup_size(4, 4, 1)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let size = textureDimensions(DESTINATION);
    if (pixel.x >= size.x || pixel.y >= size.y) { return; }

    let texel = 1.0 / vec2<f32>(size);
    let uv = (vec2<f32>(pixel.xy) + 0.5) * texel;

    let depth = textureSampleLevel(DEPTH, DEPTH_SAMPLER, uv, 0.0).r;
    let coc = abs(depth - ENVIRONMENT.settings.focal_distance) * ENVIRONMENT.settings.aperture;
    let step = DIRECTION * texel * coc;

    var result = textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv, 0.0).rgb * N_1;

    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv + step * 1.0, 0.0).rgb * N_2;
    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv - step * 1.0, 0.0).rgb * N_2;

    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv + step * 2.0, 0.0).rgb * N_3;
    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv - step * 2.0, 0.0).rgb * N_3;

    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv + step * 3.0, 0.0).rgb * N_4;
    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv - step * 3.0, 0.0).rgb * N_4;

    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv + step * 4.0, 0.0).rgb * N_5;
    result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv - step * 4.0, 0.0).rgb * N_5;

    textureStore(DESTINATION, pixel.xy, vec4<f32>(result, 1.0));
}
