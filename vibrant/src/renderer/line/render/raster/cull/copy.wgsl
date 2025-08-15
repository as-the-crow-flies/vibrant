@group(0) @binding(0) var COLOR: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var OPACITY: texture_storage_2d<r32float, read_write>;

@compute
@workgroup_size(32, 32)
fn main(@builtin(global_invocation_id) pixel: vec3<u32>) {
    let min_opacity = min(min(min(
        textureLoad(COLOR, 2u * pixel.xy + vec2<u32>(0, 0), 0).a,
        textureLoad(COLOR, 2u * pixel.xy + vec2<u32>(0, 1), 0).a),
        textureLoad(COLOR, 2u * pixel.xy + vec2<u32>(1, 0), 0).a),
        textureLoad(COLOR, 2u * pixel.xy + vec2<u32>(1, 1), 0).a);

    textureStore(OPACITY, pixel.xy, vec4<f32>(f32(min_opacity > 0.95)));
}
