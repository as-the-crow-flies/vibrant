@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(2) var DESTINATION: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(32, 32)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let sample = (2.0 * vec2<f32>(pixel.xy) + 1.0) / vec2<f32>(textureDimensions(SOURCE));
    let opacity_threshold = f32(textureSampleLevel(SOURCE, SAMPLER, sample, 0.0).x == 1.0);
    textureStore(DESTINATION, pixel.xy, vec4<f32>(opacity_threshold));
}
