@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

@group(2) @binding(0) var<uniform> CAMERA: mat4x4<f32>;

const ONE_OVER_U32_MAX: f32 = 0.000000000232831;

struct Vertex {
    @location(0) position: vec3<f32>
}

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> Fragment
{
    let position = TRACTOGRAM_TO_WORLD * vec4<f32>(vertex.position, 1.0);
    let clip = CAMERA * position;

    return Fragment(clip, position.xyz);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let opacity = 1.0 - textureSampleLevel(DENSITY, SAMPLER, fragment.position + 0.5, 5.0).x;

    // return vec4<f32>(normalize(fwidth(fragment.position)), 1.0);
    return vec4<f32>(vec3<f32>(opacity), 1.0);
}
