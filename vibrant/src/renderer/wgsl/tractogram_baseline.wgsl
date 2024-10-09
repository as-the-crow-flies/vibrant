@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
    let clip = ENVIRONMENT.camera.projection * position;
    return Fragment(clip, position.xyz);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    return vec4<f32>(normalize(fwidth(fragment.position)), 1.0);
}
