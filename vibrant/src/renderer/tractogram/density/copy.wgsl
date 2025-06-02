@group(0) @binding(0) var<storage, read_write> BUFFER: array<u32>;
@group(1) @binding(0) var DENSITY: texture_storage_3d<r8unorm, read_write>;
@group(2) @binding(0) var COUNT: texture_storage_3d<r8uint, read_write>;
@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let index = block_index(voxel, textureDimensions(DENSITY));

    let buffer = BUFFER[index];

    let density = precision_encode(saturate(U20_MAX_INV * f32(buffer >> 8)));
    let count = buffer & 255;

    textureStore(DENSITY, voxel, vec4<f32>(density));
    textureStore(COUNT, voxel, vec4<u32>(count));
}
