struct Camera {
    transform: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>
}

@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

@group(2) @binding(0) var<uniform> CAMERA: Camera;

const PI: f32 = 3.1415926535897932;
const PHI = 1.6180339887498948482045868;

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
}

@vertex
fn vertex(@location(0) vertex: vec3<f32>) -> Fragment
{
    let position = TRACTOGRAM_TO_WORLD * vec4<f32>(vertex, 1.0);
    return Fragment(CAMERA.projection * position, position.xyz);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let camera = CAMERA.transform[3].xyz;
    let L = vec3<f32>(0.0, 1.0, 0.0);

    let V = normalize(camera - fragment.position);
    let T = normalize(fwidth(fragment.position));

    // Stalling et al. 1997
    // Fast Display of Illuminated Field Lines
    let LN = sqrt(1.0 - pow(dot(L, T), 2.0));
    let VN = sqrt(1.0 - pow(dot(V, T), 2.0));
    let VR = LN * VN - dot(L, T) * dot(V, T);
    let I = .4 + .4 * LN + .2 * pow(VR, 16.0);

    let N_SAMPLES = 10.0;
    let CONE_ANGLE = tan(2.0 * PI / N_SAMPLES);
    let DIM = f32(textureDimensions(DENSITY).x);

    var lighting = 0.0;

    // Fibbonacci Sphere
    for (var i = 0.0; i < N_SAMPLES; i += 1.0) {
        let y = i / (N_SAMPLES - 1.0) * 2.0;
        let radius = sqrt(1.0 - y * y);
        let theta = PHI * i;

        let direction = vec3<f32>(cos(theta), y, sin(theta));

        var occlusion = 0.0;

        for (var distance = 0.01; distance < 1.0; distance *= 1.5) {
            let sample = fragment.position + direction * distance;
            let level = log2(2.0 * CONE_ANGLE * distance * DIM);

            occlusion += (1.0 - occlusion) * textureSampleLevel(DENSITY, SAMPLER, sample + 0.5, level).x;
        }

        lighting += (1.0 - occlusion);
    }

    lighting *= I / N_SAMPLES;

    return vec4<f32>(vec3<f32>(lighting), 1.0);
    // return vec4<f32>(lighting, 1.0);
}
