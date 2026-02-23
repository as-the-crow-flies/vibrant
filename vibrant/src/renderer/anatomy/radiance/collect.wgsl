@group(3) @binding(0) var IRRADIANCE: texture_storage_3d<rgba16float, write>;
@group(3) @binding(1) var CASCADE: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    let uv = (vec3<f32>(texel) + 0.5) / vec3<f32>(textureDimensions(IRRADIANCE));
    let irradiance = textureSampleLevel(CASCADE, SAMPLER, uv, 0.0);

    textureStore(IRRADIANCE, texel, irradiance);
}
