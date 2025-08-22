@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(0) @binding(2) var DESTINATION: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(32, 32)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let min_depth = min(min(min(
        textureLoad(SOURCE, 2u * pixel.xy + vec2<u32>(0, 0), 0).x,
        textureLoad(SOURCE, 2u * pixel.xy + vec2<u32>(0, 1), 0).x),
        textureLoad(SOURCE, 2u * pixel.xy + vec2<u32>(1, 0), 0).x),
        textureLoad(SOURCE, 2u * pixel.xy + vec2<u32>(1, 1), 0).x);

    textureStore(DESTINATION, pixel.xy, vec4<f32>(min_depth));
}
