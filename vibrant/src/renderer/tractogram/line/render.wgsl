// Used instead of vec3<f32> for padding reasons
struct Vertex {
    x: f32,
    y: f32,
    z: f32
}

@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<Vertex>;
@group(0) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

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
fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> Fragment {
    let index = TRACTOGRAM_INDICES[instance_index];
    let position = TRACTOGRAM_TO_WORLD * get_vertex(index + vertex_index);
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

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, 1.0);
}
