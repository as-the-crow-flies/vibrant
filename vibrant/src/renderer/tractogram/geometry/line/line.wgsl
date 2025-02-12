@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(1) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec4<f32>,
}

struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> Fragment {
    let index = TRACTOGRAM_INDICES[instance_index];
    let position = TRACTOGRAM_TO_WORLD * TRACTOGRAM_VERTICES[index + vertex_index];
    return Fragment(ENVIRONMENT.camera.projection * position, position);
}

@fragment
fn fragment(fragment: Fragment) -> GBuffer {
    let clip = ENVIRONMENT.camera.projection * fragment.position;
    let ndc = clip.xyz / clip.w;

    return GBuffer(
        vec4<f32>(fragment.position.xyz, 1.0),
        vec4<f32>(vec3<f32>(ndc * 0.5 + 0.5), 0.0),
        vec4<f32>(abs(normalize(fwidth(fragment.position.xyz))), 1.0)
    );
}
