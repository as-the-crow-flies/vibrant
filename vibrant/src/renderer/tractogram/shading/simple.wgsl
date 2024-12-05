@group(0) @binding(0) var POSITION: texture_2d<f32>;
@group(0) @binding(1) var NORMAL: texture_2d<f32>;
@group(0) @binding(2) var TANGENT: texture_2d<f32>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let position = textureLoad(POSITION, vec2<u32>(uv.xy), 0);
    let tangent = textureLoad(TANGENT, vec2<u32>(uv.xy), 0);

    if (position.w == 0.0) { discard; }

    let tangent_object_space = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(tangent.xyz, 0.0));

    return vec4<f32>(abs(tangent_object_space.xyz), 1.0);
}
