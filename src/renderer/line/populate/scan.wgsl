use block;

@group(0) @binding(0) var<storage, read_write> OFFSET: array<u32>;
@group(0) @binding(1) var<storage, read_write> OFFSET_TOTAL: atomic<u32>;

@group(1) @binding(0) var COUNT: texture_3d<u32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(COUNT, 0);

    if (any(voxel >= dim)) { return; }

    let count = textureLoad(COUNT, voxel, 0).x;

    if (count > 0u) {
        OFFSET[block_index(voxel, dim)] = atomicAdd(&OFFSET_TOTAL, count);
    }
}
