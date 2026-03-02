@group(2) @binding(0) var IRRADIANCE: texture_storage_3d<rgba32float, write>;
@group(2) @binding(1) var CASCADE_SAMPLER: sampler;
@group(2) @binding(2) var CASCADE_IN_POS_X: texture_3d<f32>;
@group(2) @binding(3) var CASCADE_IN_POS_Y: texture_3d<f32>;
@group(2) @binding(4) var CASCADE_IN_POS_Z: texture_3d<f32>;
@group(2) @binding(5) var CASCADE_IN_NEG_X: texture_3d<f32>;
@group(2) @binding(6) var CASCADE_IN_NEG_Y: texture_3d<f32>;
@group(2) @binding(7) var CASCADE_IN_NEG_Z: texture_3d<f32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    let uv = (vec3<f32>(texel) + 0.5) / vec3<f32>(textureDimensions(IRRADIANCE));

    let extinction = textureSampleLevel(EXTINCTION, SAMPLER, uv, 0.0).rgb;
    let transmission = exp(-STEP_SIZE * extinction);

    let irradiance =
        textureSampleLevel(CASCADE_IN_POS_X, CASCADE_SAMPLER, uv, 0.0) +
        textureSampleLevel(CASCADE_IN_NEG_X, CASCADE_SAMPLER, uv, 0.0) +
        textureSampleLevel(CASCADE_IN_POS_Y, CASCADE_SAMPLER, uv, 0.0) +
        textureSampleLevel(CASCADE_IN_NEG_Y, CASCADE_SAMPLER, uv, 0.0) +
        textureSampleLevel(CASCADE_IN_POS_Z, CASCADE_SAMPLER, uv, 0.0) +
        textureSampleLevel(CASCADE_IN_NEG_Z, CASCADE_SAMPLER, uv, 0.0);

    textureStore(IRRADIANCE, texel, vec4<f32>(transmission, 0.0) * irradiance);
}
