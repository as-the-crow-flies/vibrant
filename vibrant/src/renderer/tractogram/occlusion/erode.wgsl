@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(1) @binding(0) var DESTINATION: texture_storage_2d<r32float, read_write>;

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel = vec2<i32>(id.xy);

    let maximum = max(max(max(max(max(max(max(max(
        textureLoad(SOURCE, pixel + vec2<i32>(-1,-1), 0),
        textureLoad(SOURCE, pixel + vec2<i32>(-1, 0), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>(-1, 1), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>( 0,-1), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>( 0, 0), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>( 0, 1), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>( 1,-1), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>( 1, 0), 0)),
        textureLoad(SOURCE, pixel + vec2<i32>( 1, 1), 0));

    textureStore(DESTINATION, id.xy, vec4<f32>(maximum));
}
