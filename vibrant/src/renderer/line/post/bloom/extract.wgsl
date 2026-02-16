@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var COLOR: texture_2d<f32>;
@group(1) @binding(1) var COLOR_SAMPLER: sampler;
@group(2) @binding(0) var BLOOM: texture_storage_2d<rgba16float, write>;

// https://openaccess.thecvf.com/content/ICCV2025/papers/Wang_From_Abyssal_Darkness_to_Blinding_Glare_A_Benchmark_on_Extreme_ICCV_2025_paper.pdf
// L = 0.2126R + 0.7152G + 0.0722B
const LUMINANCE_R: f32 = 0.2126;
const LUMINANCE_G: f32 = 0.7152;
const LUMINANCE_B: f32 = 0.0722;

@compute
@workgroup_size(4, 4, 1)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let size = textureDimensions(BLOOM);
    if (pixel.x >= size.x || pixel.y >= size.y) { return; }

    let uv = (vec2<f32>(pixel.xy) + 0.5) / vec2<f32>(size);
    let color = textureSampleLevel(COLOR, COLOR_SAMPLER, uv, 0.0);

    let threshold = ENVIRONMENT.settings.bloom_threshold;
    let knee = threshold * 0.4;

    let luminance = dot(color.rgb, vec3<f32>(LUMINANCE_R, LUMINANCE_G, LUMINANCE_B));

    // Soft knee extraction
    let soft = luminance - threshold + knee;
    let contribution = clamp(soft * soft / (4.0 * knee + 0.00001), 0.0, 1.0);
    let factor = max(contribution, luminance - threshold) / max(luminance, 0.0001);

    textureStore(BLOOM, pixel.xy, vec4<f32>(color.rgb * factor, 1.0));
}
