@group(0) @binding(0) var OCCLUSION: texture_storage_3d<r8unorm, read_write>;
@group(0) @binding(3) var<uniform> OCCLUSION_TO_PROJECTION: mat4x4<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var DENSITY_SAMPLER: sampler;
@group(1) @binding(2) var<uniform> WORLD_TO_DENSITY: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (any(id > textureDimensions(OCCLUSION))) { return; }

    let froxel = vec3<f32>(id);
    let steps = 2;

    let step = 1.00001 / f32(steps);

    var density = 0.0;

    for (var x = 0.0; x < 1.0; x += step) {
        for (var y = 0.0; y < 1.0; y += step) {
            for (var z = 0.0; z < 1.0; z += step) {
                density += textureSampleLevel(DENSITY, DENSITY_SAMPLER, transform(froxel + vec3<f32>(x, y, z)), 0.0).x;
            }
        }
    }

    textureStore(OCCLUSION, id, vec4<f32>(1.0 / pow(f32(steps), 3.0) * density));
}

fn transform(froxel: vec3<f32>) -> vec3<f32> {
    let transform = ENVIRONMENT.camera.projection_inverse * OCCLUSION_TO_PROJECTION;
    let sample = transform * vec4<f32>(froxel, 1.0);
    return sample.xyz / sample.w + 0.5;
}
