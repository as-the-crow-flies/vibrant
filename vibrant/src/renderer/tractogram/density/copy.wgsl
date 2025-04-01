@group(0) @binding(0) var<storage, read_write> BUFFER: array<u32>;
@group(1) @binding(0) var TEXTURE: texture_storage_3d<r8unorm, read_write>;
@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const ONE_OVER_U16_MAX: f32 = 0.0000152590219;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(TEXTURE);
    let index = linear_index(voxel);

    let density = saturate(ONE_OVER_U16_MAX * f32(BUFFER[index]));
    textureStore(TEXTURE, voxel, vec4<f32>(density, 0.0, 0.0, 1.0));
}
