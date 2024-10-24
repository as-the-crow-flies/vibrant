@group(0) @binding(0) var<storage, read_write> VOLUME:
    array<array<array<u32, VOLUME_XYZ>, VOLUME_XYZ>, VOLUME_XYZ>;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= vec3<u32>(VOLUME_XYZ))) { return; }

    VOLUME[voxel.z][voxel.y][voxel.x] = 0u;
}
