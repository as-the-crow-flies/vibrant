@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(2) var DESTINATION: texture_storage_2d<rgba16float, write>;

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    let uv = (2.0 * vec2<f32>(texel.xy) + 1.0) / vec2<f32>(textureDimensions(SOURCE));
    textureStore(DESTINATION, texel.xy, textureSampleLevel(SOURCE, SAMPLER, uv, 0.0));
}
