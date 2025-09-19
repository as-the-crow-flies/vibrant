@group(0) @binding(0) var<storage, read_write> MORTON: array<u32>;
@group(0) @binding(2) var<storage, read_write> COUNT: u32;
@group(0) @binding(3) var<storage, read_write> OFFSET: atomic<u32>;

@group(1) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(1) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> WORKGROUP_OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let count = COUNT;

    var offset = 0u;

    while (offset < count) {
        if (local == 0) {
            WORKGROUP_OFFSET = atomicAdd(&OFFSET, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&WORKGROUP_OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = offset + i * WORKGROUP_SIZE + local;

            if (index == 0u || index >= count) { continue; }

            let voxel_bits_0 = MORTON[index - 1u];
            let voxel_bits_1 = MORTON[index];

            if (voxel_bits_0 != voxel_bits_1) {
                textureStore(END, morton_decode(voxel_bits_0), vec4<u32>(index));
                textureStore(START, morton_decode(voxel_bits_1), vec4<u32>(index));
            }
        }
    }
}
