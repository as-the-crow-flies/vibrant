@group(0) @binding( 0) var SAMPLER: sampler;
@group(0) @binding( 1) var IRRADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding( 2) var IMPORTANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding( 3) var RADIANCE_0: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    if (any(voxel >= textureDimensions(IRRADIANCE))) { return; }

    let dim = textureDimensions(IRRADIANCE);
    let dim_inv = 1.0 / vec3<f32>(textureDimensions(RADIANCE_0));

    var irradiance = vec3<f32>(0.0);

    for (var dx=0u; dx<2u; dx++) {
        for (var dy=0u; dy<2u; dy++) {
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

fn importance(a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, d: vec4<f32>) -> vec4<f32> {
    return normed(vec4<f32>(
        brightness(unpack_rgb(a)),
        brightness(unpack_rgb(b)),
        brightness(unpack_rgb(c)),
        brightness(unpack_rgb(d))
    ));
}

fn normed(v: vec4<f32>) -> vec4<f32> {
    return v / max(v.x, max(v.y, max(v.z, v.w)));
}

fn brightness(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}
