@group(0) @binding(0) var IRRADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var RADIANCE_0: texture_3d<f32>;
@group(0) @binding(2) var SAMPLER: sampler;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(IRRADIANCE))) { return; }

    let dim = textureDimensions(IRRADIANCE);
    let dim_inv = 1.0 / vec3<f32>(textureDimensions(RADIANCE_0));

    var irradiance = vec3<f32>(0.0);

    for (var dx=0u; dx<3u; dx++) {
        for (var dy=0u; dy<3u; dy++) {
            for (var jx=-1; jx<=1; jx+=2) {
                for (var jy=-1; jy<=1; jy+=2) {
                    for (var jz=-1; jz<=1; jz+=2) {
                        let direction = dim * vec3<u32>(dx, dy, 0);
                        let jitter = vec3<i32>(jx, jy, jz);

                        let sample = dim_inv * (vec3<f32>(voxel + direction) + 0.5 + vec3<f32>(jitter));
                        irradiance += unpack_rgb(textureSampleLevel(RADIANCE_0, SAMPLER, sample, 0.0));
                    }
                }
            }
        }
    }

    textureStore(IRRADIANCE, voxel, pack_rgb(irradiance / 8.0));
}
