@group(0) @binding(3) var<storage, read_write> BIN: array<atomic<u32>>;
@group(1) @binding(0) var COUNT: texture_3d<u32>;
@group(2) @binding(0) var OCCLUSION: texture_3d<f32>;
@group(2) @binding(1) var OCCLUSION_SAMPLER: sampler;

var<workgroup> WORKGROUP_BIN: array<atomic<u32>, U8_MAX>;

@compute
@workgroup_size(8, 8, 8)
fn main(
    @builtin(global_invocation_id) voxel: vec3<u32>,
    @builtin(local_invocation_index) local: u32
) {
    let dim = textureDimensions(COUNT);
    let uv = vec3<f32>(voxel) / vec3<f32>(dim);

    let count = textureLoad(COUNT, voxel, 0).x;
    let occlusion = textureSampleLevel(OCCLUSION, OCCLUSION_SAMPLER, uv, 0.0).x;

    let bin = u32(0.5 * saturate(occlusion) * U8_MAX_f32);

    atomicAdd(&WORKGROUP_BIN[bin], count);

    workgroupBarrier();

    if (local < U8_MAX) {
        atomicAdd(&BIN[local], atomicLoad(&WORKGROUP_BIN[local]));
    }
}
