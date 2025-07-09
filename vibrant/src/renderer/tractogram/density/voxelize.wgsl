@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> COLOR: array<atomic<u32>>;

@group(1) @binding(2) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;

@group(2) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(2) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(2) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;
@group(2) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(4) var<storage, read_write> TRACTOGRAM_COUNT: atomic<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&TRACTOGRAM_INDICES);
    let radius = ENVIRONMENT.settings.streamline_radius;
    let TRANSFORM = WORLD_TO_VOLUME;

    loop {
        if (local == 0) {
            OFFSET = atomicAdd(&TRACTOGRAM_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        let offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { return; }

            let index = TRACTOGRAM_INDICES[index_index];

            let v0 = transform(TRANSFORM, TRACTOGRAM_VERTICES[index + 0]);
            let v1 = transform(TRANSFORM, TRACTOGRAM_VERTICES[index + 1]);

            voxelize(index, v0, v1, radius);
        }
    }
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: vec3<f32>, v1: vec3<f32>) {
    let smoothing = ENVIRONMENT.settings.smoothing;
    let radius = ENVIRONMENT.settings.streamline_radius;
    let radius_clamp = max(smoothing, radius);
    let radius_ratio = radius;

    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));

    let coverage_multiplier = radius_ratio * U16_MAX_f32;

    let sample = vec3<f32>(voxel) + 0.5;

    // TODO: endpoint spheres are now voxelized twice, take normal planes into account
    let coverage = saturate(0.5 - capsule(sample, v0, v1, radius_clamp));
    let alpha = ENVIRONMENT.settings.alpha * coverage;

    let density_encoded = u32(alpha * coverage_multiplier) << U14_SHIFT;
    let count_encoded = 1u;

    atomicAdd(&DENSITY[idx], density_encoded + count_encoded);

    if (ENVIRONMENT.settings.alpha == 1.0) { return; }

    let tangent = normalize(v1 - v0);
    let light = ENVIRONMENT.light;

    let diffuse = mix(1.0, stalling(tangent, light), ENVIRONMENT.settings.direct_light);
    let color = alpha * diffuse * abs(tangent);
    let color_encoded = vec3<u32>(color * U24_MAX_f32);

    atomicAdd(&COLOR[idx * 3 + 0], color_encoded.x);
    atomicAdd(&COLOR[idx * 3 + 1], color_encoded.y);
    atomicAdd(&COLOR[idx * 3 + 2], color_encoded.z);
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
