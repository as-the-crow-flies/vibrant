@group(0) @binding(0) var IRRADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var RADIANCE_0: texture_3d<f32>;
@group(0) @binding(2) var SAMPLER: sampler;

fn sample_octant(voxel: vec3<f32>, octant: vec3<f32>, rad_dim: vec3<f32>) -> vec4<f32> {
    let uv = (voxel + octant) / rad_dim;
    let d  = 0.5 / rad_dim;
    return 0.5 * (textureSampleLevel(RADIANCE_0, SAMPLER, uv + d, 0.0) +
                  textureSampleLevel(RADIANCE_0, SAMPLER, uv - d, 0.0));
}

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = textureDimensions(IRRADIANCE);

    if (any(voxel >= dim)) { return; }

    let rad_dim = vec3<f32>(textureDimensions(RADIANCE_0));
    let vf      = vec3<f32>(voxel) + 0.5;
    let ox      = vec3<f32>(f32(dim.x), 0.0,         0.0        );
    let oy      = vec3<f32>(0.0,         f32(dim.y), 0.0        );
    let oz      = vec3<f32>(0.0,         0.0,         f32(dim.z));

    let irradiance = 0.125 * (
        sample_octant(vf, vec3(0.0)     , rad_dim) +
        sample_octant(vf, ox            , rad_dim) +
        sample_octant(vf, oy            , rad_dim) +
        sample_octant(vf, oz            , rad_dim) +
        sample_octant(vf, ox + oy       , rad_dim) +
        sample_octant(vf, ox + oz       , rad_dim) +
        sample_octant(vf, oy + oz       , rad_dim) +
        sample_octant(vf, ox + oy + oz  , rad_dim));

    textureStore(IRRADIANCE, voxel, pack_rgb(irradiance.rgb));
}
