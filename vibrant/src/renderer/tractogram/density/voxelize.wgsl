@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;
@group(1) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(workgroup_id) workgroup: vec3<u32>,
    @builtin(local_invocation_index) local: u32
) {
    let radius = ENVIRONMENT.settings.streamline_radius;
    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;

    let n_indices = arrayLength(&TRACTOGRAM_INDICES);

    let offset = workgroup.x * WORKGROUP_SIZE * CHUNK_SIZE;

    for (var chunk=0u; chunk<CHUNK_SIZE; chunk++) {

        let index_index = offset + chunk * WORKGROUP_SIZE + local;

        if (index_index >= n_indices - 1) { continue; }

        let index = TRACTOGRAM_INDICES[index_index];

        let v0 = (transform * TRACTOGRAM_VERTICES[index    ]).xyz;
        let v1 = (transform * TRACTOGRAM_VERTICES[index + 1]).xyz;

        voxelize(index, v0, v1, radius);
    }
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: vec3<f32>, v1: vec3<f32>) {
    let smoothing = ENVIRONMENT.settings.smoothing;
    let radius = ENVIRONMENT.settings.streamline_radius;
    let radius_clamp = max(smoothing, radius);

    let coverage_multiplier = saturate(radius * radius / smoothing) * f32(U20_MAX);

    let sample = vec3<f32>(voxel) + 0.5;

    let alpha = saturate(0.5 - capsule(sample, v0, v1, radius_clamp))
              - saturate(0.5 - sphere(sample - v0, radius_clamp));

    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));
    let value = u32(ENVIRONMENT.settings.alpha * alpha * coverage_multiplier);

    atomicAdd(&DENSITY[idx], (value << 8) + 1);
}

fn capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

fn sphere(p: vec3<f32>, r: f32) -> f32 {
  return length(p) - r;
}
