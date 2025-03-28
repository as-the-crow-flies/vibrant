fn linear_index(voxel: vec3<u32>) -> u32 {
    // let stride = vec3<u32>(ENVIRONMENT.volume * ENVIRONMENT.volume, ENVIRONMENT.volume, 1u);
    // return dot(voxel, stride);

    let BLOCK_SIZE = 8u;
    let BLOCKS_PER_DIM = ENVIRONMENT.volume / BLOCK_SIZE;
    let BLOCK_VOLUME = BLOCK_SIZE * BLOCK_SIZE * BLOCK_SIZE;

    let block = dot(voxel / BLOCK_SIZE, vec3<u32>(BLOCKS_PER_DIM * BLOCKS_PER_DIM, BLOCKS_PER_DIM, 1));
    let offset = dot(voxel % BLOCK_SIZE, vec3<u32>(BLOCK_SIZE * BLOCK_SIZE, BLOCK_SIZE, 1));

    return block * BLOCK_VOLUME + offset;
}
