@group(0) @binding(0) var SOURCE: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(0) @binding(2) var DESTINATION: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel = 2 * id.xy;

    let minimum = min(min(min(
        textureLoad(SOURCE, pixel + vec2<u32>(0, 0), 0).x,
        textureLoad(SOURCE, pixel + vec2<u32>(0, 1), 0).x),
        textureLoad(SOURCE, pixel + vec2<u32>(1, 0), 0).x),
        textureLoad(SOURCE, pixel + vec2<u32>(1, 1), 0).x);

    textureStore(DESTINATION, id.xy, vec4<f32>(minimum));
}
