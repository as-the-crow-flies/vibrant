@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
}

struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
}

@vertex
fn vertex(@location(0) vertex: vec3<f32>) -> Fragment
{
    let position = TRACTOGRAM_TO_WORLD * vec4<f32>(vertex, 1.0);
    return Fragment(ENVIRONMENT.camera.projection * position, position.xyz);
}

@fragment
fn fragment(fragment: Fragment) -> GBuffer {
    return GBuffer(
        vec4<f32>(fragment.position, 1.0),
        vec4<f32>(0.0),
        vec4<f32>(normalize(fwidth(fragment.position)) * 0.5 + 0.5, 1.0)
    );
}
