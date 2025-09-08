@group(0) @binding(0) var CULLING: texture_storage_3d<r32float, read_write>;

@group(1) @binding(0) var<storage, read_write> OFFSET: array<u32>;
@group(1) @binding(1) var<storage, read_write> OFFSET_TOTAL: atomic<u32>;

@group(2) @binding(0) var COUNT: texture_3d<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(CULLING).x;

    let culling = textureLoad(CULLING, voxel).x;

    if (culling > 0.0) {
        let count = textureLoad(COUNT, voxel, 0).x;
        OFFSET[block_index(voxel, vec3<u32>(dim))] = atomicAdd(&OFFSET_TOTAL, count);
    }
}
