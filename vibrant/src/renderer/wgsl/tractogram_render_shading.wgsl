struct Camera {
    transform: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>
}

@group(0) @binding(0) var GBUFFER: texture_2d<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

@group(2) @binding(0) var<uniform> CAMERA: Camera;

const PI: f32 = 3.1415926535897932;
const PHI = 1.6180339887498948482045868;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32>
{
    return vec4<f32>(2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let camera = CAMERA.transform[3].xyz;
    let L = vec3<f32>(0.0, 1.0, 0.0);

    let gbuffer = textureLoad(GBUFFER, vec2<u32>(uv.xy), 0);

    if (all(gbuffer == vec4<f32>(0.0, 0.0, 0.0, 1.0))) { discard; }

    let position = gbuffer.xyz;
    let T = unpack4x8snorm(bitcast<u32>(gbuffer.w)).xyz;

    let V = normalize(camera - position);

    // Stalling et al. 1997
    // Fast Display of Illuminated Field Lines
    let LN = sqrt(1.0 - pow(dot(L, T), 2.0));
    let VN = sqrt(1.0 - pow(dot(V, T), 2.0));
    let VR = LN * VN - dot(L, T) * dot(V, T);
    let I = .4 + .4 * LN + .2 * pow(VR, 16.0);

    let N_SAMPLES = 10.0;
    let TAN_CONE_ANGLE = tan(2.0 * PI / N_SAMPLES);
    let DIM = f32(textureDimensions(DENSITY).x);

    var lighting = 0.0;

    // Fibbonacci Sphere
    for (var i = 0.0; i < N_SAMPLES; i += 1.0) {
        let y = i / (N_SAMPLES - 1.0) * 2.0;
        let radius = sqrt(1.0 - y * y);
        let theta = PHI * i;

        let direction = vec3<f32>(cos(theta), y, sin(theta));

        var occlusion = 0.0;

        for (var distance = 1.0 / DIM; distance < 1.0; distance *= 2.0) {
            let sample = position + direction * distance;
            let level = log2(2.0 * TAN_CONE_ANGLE * distance * DIM);

            occlusion += (1.0 - occlusion) * textureSampleLevel(DENSITY, SAMPLER, sample + 0.5, level).x;
        }

        lighting += (1.0 - occlusion);
    }

    lighting *= I / N_SAMPLES;

    return vec4<f32>(vec3<f32>(lighting), 1.0);
    // return vec4<f32>(lighting, 1.0);
}
