@group(0) @binding(0) var<storage, read_write> DENSITY_COUNT_BUFFER: array<u32>;
@group(0) @binding(1) var<storage, read_write> TANGENT_BUFFER: array<u32>;

@group(1) @binding(0) var DENSITY: texture_storage_3d<r32float, read_write>;
@group(2) @binding(0) var COUNT: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(0) var TANGENT: texture_storage_3d<rgba8unorm, read_write>;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let index = block_index(voxel, textureDimensions(DENSITY));

    let density_count_encoded = DENSITY_COUNT_BUFFER[index];
    let density = U12_MAX_INV * f32(density_count_encoded >> U14_SHIFT);
    let count = density_count_encoded & U14_MAX;

    textureStore(DENSITY, voxel, vec4<f32>(density));
    textureStore(COUNT, voxel, vec4<u32>(count));

    let tangent_encoded = TANGENT_BUFFER[index];
    let t = vec2<f32>(vec2<u32>(tangent_encoded >> 16, tangent_encoded & U16_MAX)) * U12_MAX_INV;
    let tangent = normalize(vec3<f32>(t, sqrt(density * density - t.x * t.x - t.y * t.y)));

    textureStore(TANGENT, voxel, vec4<f32>(tangent, density));
}
