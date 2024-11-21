@group(0) @binding(0) var SRC: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;
@group(1) @binding(0) var DST: texture_storage_3d<r32float, write>;
@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;
@group(3) @binding(0) var<uniform> STEP: f32;

@compute
@workgroup_size(WORKGROUP_XYZ, WORKGROUP_XYZ, WORKGROUP_XYZ)
fn compute(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let one_over_dim = 1.0 / f32(VOLUME_XYZ);
    let half = vec3<f32>(0.5);

    var result = textureLoad(SRC, voxel, 0).x;

    // let camera = 10.0 * ENVIRONMENT.light;
    let camera = ENVIRONMENT.camera.transform[3].xyz;
    let position = vec3<f32>(voxel) * one_over_dim - half + half * one_over_dim;

    let delta = camera - position;
    let distance = length(delta);
    let direction = normalize(delta);

    let sample = textureSampleLevel(SRC, SAMPLER, position + direction * STEP + half, 0.0).x;

    if (STEP < distance) { result += sample; }

    textureStore(DST, voxel, vec4<f32>(vec3<f32>(result), 1.0));
}
