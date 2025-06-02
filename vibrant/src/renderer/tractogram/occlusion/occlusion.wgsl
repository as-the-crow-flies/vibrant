@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var DENSITY_SAMPLER: sampler;

@group(1) @binding(0) var OCCLUSION: texture_storage_3d<r8unorm, read_write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = vec3<f32>(textureDimensions(DENSITY));

    // Camera Position in World Space
    let camera = ENVIRONMENT.camera.transform[3].xyz;

    // let camera = vec3<f32>(0.0, 0.0, 1.0);

    // Voxel Position in World Space
    let position = vec3<f32>(voxel) / vec3<f32>(textureDimensions(OCCLUSION)) - 0.5;

    let delta = camera - position;
    let distance = length(delta);

    let direction = delta / distance;
    let step = 1.0 / maximum(abs(direction * dim));

    let factor = distance * step * dim.x;

    var absorbance = 0.0;

    for (var depth = 0.0; depth < 1.0; depth += step) {
        let sample = mix(position, camera, depth);

        if (any(abs(sample) > vec3<f32>(0.5))) { break; }

        absorbance += factor * density(sample);
    }

    textureStore(OCCLUSION, voxel, vec4<f32>(absorbance));
}

fn density(sample: vec3<f32>) -> f32 {
    return precision_decode(textureSampleLevel(DENSITY, DENSITY_SAMPLER, sample + 0.5, 0.0).x);
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}
