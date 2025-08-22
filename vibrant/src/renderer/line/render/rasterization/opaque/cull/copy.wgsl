@group(0) @binding(0) var DEPTH: texture_depth_2d;
@group(1) @binding(0) var HIZ: texture_storage_2d<r32float, read_write>;

@compute
@workgroup_size(32, 32)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let min_depth = min(min(min(
        textureLoad(DEPTH, 2u * pixel.xy + vec2<u32>(0, 0), 0),
        textureLoad(DEPTH, 2u * pixel.xy + vec2<u32>(0, 1), 0)),
        textureLoad(DEPTH, 2u * pixel.xy + vec2<u32>(1, 0), 0)),
        textureLoad(DEPTH, 2u * pixel.xy + vec2<u32>(1, 1), 0));

    textureStore(HIZ, pixel.xy, vec4<f32>(min_depth));
}
