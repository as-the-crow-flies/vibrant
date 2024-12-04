@group(0) @binding(0) var<storage, read_write> BUFFER: array<array<array<u32, VOLUME_XYZ>, VOLUME_XYZ>, VOLUME_XYZ>;
@group(1) @binding(0) var TEXTURE: texture_storage_3d<r32float, write>;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let density = f32(countOneBits(BUFFER[voxel.z][voxel.y][voxel.x])) / 27.0;
    textureStore(TEXTURE, voxel, vec4<f32>(vec3<f32>(density), 1.0));
}
