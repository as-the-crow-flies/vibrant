struct Camera {
    transform: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>
}

@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var<uniform> CAMERA: Camera;

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
    let tangent = vec4<f32>(normalize(fwidth(fragment.position)), 1.0);
    return vec4<f32>(fragment.position, bitcast<f32>(pack4x8snorm(tangent)));
}
