@group(0) @binding(0) var OCCLUSION: texture_storage_3d<r32float, read_write>;
@group(0) @binding(3) var<uniform> OCCLUSION_TO_PROJECTION: mat4x4<f32>;

@group(1) @binding(0) var THRESHOLD: texture_storage_2d<r32float, read_write>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel = id.xy;
    let max_depth = textureDimensions(OCCLUSION).z;

    var occlusion = 0.0;
    var threshold = 1.0;

    for (var froxel = vec3<u32>(pixel, 0); froxel.z < max_depth; froxel.z++) {
        let step = length(transform(froxel + vec3<u32>(0, 0, 1)) - transform(froxel));

        let density = textureLoad(OCCLUSION, froxel).x;

        occlusion += step * density;

        textureStore(OCCLUSION, froxel, vec4<f32>(occlusion));

        if (threshold == 1.0 && occlusion > ENVIRONMENT.settings.culling_threshold) {
            let sample = OCCLUSION_TO_PROJECTION * vec4<f32>(vec3<f32>(froxel), 1.0);
            threshold = sample.z / sample.w;
        }
    }

    textureStore(THRESHOLD, pixel, vec4<f32>(threshold));
}

fn transform(froxel: vec3<u32>) -> vec3<f32> {
    let transform = ENVIRONMENT.camera.projection_inverse * OCCLUSION_TO_PROJECTION;
    let sample = transform * vec4<f32>(vec3<f32>(froxel), 1.0);
    return sample.xyz / sample.w * vec3<f32>(f32(ENVIRONMENT.volume));
}
