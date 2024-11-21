@group(0) @binding(0) var<storage, read_write> BUFFER: array<array<array<u32, VOLUME_XYZ>, VOLUME_XYZ>, VOLUME_XYZ>;
@group(1) @binding(0) var TEXTURE: texture_storage_3d<r32float, write>;

const ONE_OVER_U32_MAX: f32 = 0.000000000232831;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= vec3<u32>(VOLUME_XYZ))) { return; }

    let density = ONE_OVER_U32_MAX * f32(BUFFER[voxel.z][voxel.y][voxel.x]);

    textureStore(TEXTURE, voxel, vec4<f32>(density, 0.0, 0.0, 1.0));
}
