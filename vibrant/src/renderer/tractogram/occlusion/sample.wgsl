@group(0) @binding(0) var OCCLUSION: texture_storage_3d<r8unorm, read_write>;
@group(0) @binding(3) var<uniform> OCCLUSION_TO_PROJECTION: mat4x4<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var DENSITY_SAMPLER: sampler;
@group(1) @binding(2) var<uniform> WORLD_TO_DENSITY: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dim = textureDimensions(OCCLUSION);

    let ratio = f32(textureDimensions(DENSITY).z) / f32(textureDimensions(OCCLUSION).z);

    if (any(id > dim)) { return; }

    let position = (vec3<f32>(id) + 0.5) / vec3<f32>(dim);
    let uv = 2.0 * position.xy - 1.0;
    let near = unproject(vec4<f32>(uv, 0.0, 1.0));
    let far = unproject(vec4<f32>(uv, 1.0, 1.0));
    let sample = mix(near, far, position.z) + 0.5;

    let value = ratio * saturate(textureSampleLevel(DENSITY, DENSITY_SAMPLER, sample, 0.0));

    textureStore(OCCLUSION, id, value);
}

fn unproject(v: vec4<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * v;
    return t.xyz / t.w;
}
