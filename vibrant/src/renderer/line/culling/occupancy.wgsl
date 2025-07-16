@group(0) @binding(0) var<storage, read_write> OFFSET: array<u32>;

@group(0) @binding(1) var<storage, read_write> SUM: atomic<u32>;
@group(0) @binding(4) var<storage, read_write> THRESHOLD: f32;

@group(1) @binding(0) var COUNT: texture_3d<u32>;
@group(2) @binding(0) var OCCLUSION: texture_3d<f32>;
@group(2) @binding(1) var OCCLUSION_SAMPLER: sampler;

@group(3) @binding(0) var OCCUPANCY: texture_storage_3d<r32float, read_write>;

var<workgroup> WORKGROUP_GLOBAL_OFFSET: u32;

@compute
@workgroup_size(512)
fn main(
    @builtin(workgroup_id) block: vec3<u32>,
    @builtin(local_invocation_index) local: u32,
    @builtin(subgroup_invocation_id) subgroup: u32,
    @builtin(subgroup_size) subgroup_size: u32,
) {
    let dim = textureDimensions(COUNT);

    let voxel = block * 8 + vec3<u32>((local >> 6) & 7, (local >> 3) & 7, local & 7);
    let uv = vec3<f32>(voxel) / vec3<f32>(dim);

    let count = textureLoad(COUNT, voxel, 0).x;
    let occlusion = 0.1 * textureSampleLevel(OCCLUSION, OCCLUSION_SAMPLER, uv, 0.0).x;

    let occupancy = u32(occlusion < THRESHOLD) * count;

    let workgroup_offset = workgroupExclusiveAdd(occupancy, local, subgroup, subgroup_size);

    if (local == 511) {
        WORKGROUP_GLOBAL_OFFSET = atomicAdd(&SUM, workgroup_offset + occupancy);
    }

    let offset = workgroupUniformLoad(&WORKGROUP_GLOBAL_OFFSET) + workgroup_offset;

    OFFSET[block_index(voxel, dim)] = offset;

    textureStore(OCCUPANCY, voxel, vec4<f32>(f32(occupancy > 0)));
}
