const BLOCK_BITS: u32 = 3u;
const BLOCK_SIZE: u32 = 8u;
const BLOCK_SIZE_2: u32 = BLOCK_SIZE * BLOCK_SIZE;
const BLOCK_SIZE_3: u32 = BLOCK_SIZE_2 * BLOCK_SIZE;

fn in_volume(voxel: vec3<i32>, dim: vec3<u32>) -> bool {
    return all(voxel >= vec3<i32>(0)) && all(vec3<u32>(voxel) < dim);
}

fn block_index(voxel: vec3<u32>, dim: vec3<u32>) -> u32 {
    // Number of blocks along each axis
    let blocks = dim >> vec3<u32>(BLOCK_BITS);

    // Block coordinate of this voxel
    let block_coord = voxel >> vec3<u32>(BLOCK_BITS);

    // Local coordinate within the block
    let local_coord = voxel & vec3<u32>(BLOCK_SIZE - 1);

    // Linear index of the block
    let block_index =
        block_coord.z * blocks.x * blocks.y +
        block_coord.y * blocks.x +
        block_coord.x;

    // Linear index within the block
    let local_index =
        local_coord.z * BLOCK_SIZE_2 +
        local_coord.y * BLOCK_SIZE +
        local_coord.x;

    // Total linear index
    return block_index * BLOCK_SIZE_3 + local_index;
}
