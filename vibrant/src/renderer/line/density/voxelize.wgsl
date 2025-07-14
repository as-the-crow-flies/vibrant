@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> COLOR: array<atomic<u32>>;

@group(1) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&LINE_INDEX);
    let radius = ENVIRONMENT.settings.streamline_radius;

    loop {
        if (local == 0) {
            OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        let offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { return; }

            let index = LINE_INDEX[index_index];
            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            voxelize(index, v0, v1, radius);
        }
    }
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    let smoothing = ENVIRONMENT.settings.smoothing;
    let radius = ENVIRONMENT.settings.streamline_radius;
    let radius_clamp = max(smoothing, radius);
    let radius_ratio = radius; // FIX ME

    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));

    let coverage_multiplier = radius_ratio * U16_MAX_f32;

    let sample = vec3<f32>(voxel) + 0.5;

    // TODO: endpoint spheres are now voxelized twice, take normal planes into account
    let sample_v0 = sample - v0.xyz;
    let delta = v1.xyz - v0.xyz;
    let height = clamp(dot(sample_v0, delta) / dot(delta, delta), 0.0, 1.0);
    let signed_distance = length(sample_v0 - delta * height) - radius_clamp;

    let alpha = ENVIRONMENT.settings.alpha * mix(v0.alpha, v1.alpha, height) * saturate(0.5 - signed_distance);

    let density_encoded = u32(alpha * coverage_multiplier) << U14_SHIFT;

    atomicAdd(&DENSITY[idx], density_encoded + 1u);
}

fn sphere(p: vec3<f32>, r: f32) -> f32 {
  return length(p) - r;
}
