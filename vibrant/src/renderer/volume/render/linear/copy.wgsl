@group(0) @binding( 0) var SAMPLER: sampler;
@group(0) @binding( 1) var IRRADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding( 2) var RADIANCE_0: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(IRRADIANCE))) { return; }

    // textureStore(IRRADIANCE, voxel, textureLoad(RADIANCE_0, voxel, 0));

    let dim_inv = 1.0 / vec3<f32>(textureDimensions(IRRADIANCE));

    var irradiance = vec3<f32>(0.0);

    for (var jx=-1; jx<=1; jx+=2) {
        for (var jy=-1; jy<=1; jy+=2) {
            for (var jz=-1; jz<=1; jz+=2) {
                let jitter = vec3<i32>(jx, jy, jz);

                let sample = dim_inv * (vec3<f32>(voxel) + 0.5 + vec3<f32>(jitter));
                irradiance += unpack_rgb(textureSampleLevel(RADIANCE_0, SAMPLER, sample, 0.0));
            }
        }
    }

    textureStore(IRRADIANCE, voxel, pack_rgb(irradiance / 2.0));
}
