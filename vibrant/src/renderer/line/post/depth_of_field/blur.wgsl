@group(0) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(1) @binding(0) var SOURCE: texture_2d<f32>;
@group(1) @binding(1) var SOURCE_SAMPLER: sampler;
@group(2) @binding(0) var DESTINATION: texture_storage_2d<rgba16float, write>;
@group(3) @binding(0) var DEPTH: texture_2d<f32>;
@group(3) @binding(1) var DEPTH_SAMPLER: sampler;

const SIGMA: f32 = 5.0;
const RADIUS: i32 = 16;

fn gaussian(x: f32) -> f32 {
    return exp(-(x * x) / (2.0 * SIGMA * SIGMA));
}

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

    var total_weight: f32 = 0.0;
    var result = vec3<f32>(0.0);

    for (var i: i32 = -RADIUS; i <= RADIUS; i++) {
        let w = gaussian(f32(i));
        result += textureSampleLevel(SOURCE, SOURCE_SAMPLER, uv + step * f32(i), 0.0).rgb * w;
        total_weight += w;
    }

    result /= total_weight;

    textureStore(DESTINATION, pixel.xy, vec4<f32>(result, 1.0));
}
