@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec3<f32>>;
@group(0) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(1) @binding(0) var<storage, read_write> VOLUME:
    array<array<array<atomic<u32>, VOLUME_XYZ>, VOLUME_XYZ>, VOLUME_XYZ>;
@group(1) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;
@group(1) @binding(2) var<uniform> VOLUME_TO_WORLD: mat4x4<f32>;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn clear(@builtin(global_invocation_id) voxel: vec3<u32>) {
    atomicStore(&VOLUME[voxel.z][voxel.y][voxel.x], 0u);
}

@compute
@workgroup_size(WORKGROUP_X)
fn rasterize(@builtin(global_invocation_id) id: vec3<u32>) {
}
