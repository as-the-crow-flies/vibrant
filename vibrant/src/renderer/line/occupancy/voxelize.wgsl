@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;

@group(1) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

const PI: f32 = 3.14159265358979323846264338327950288;

var<workgroup> OFFSET: u32;

var<private> RADIUS: f32;
var<private> DENSITY_MULTIPLIER: f32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&LINE_INDEX);

    RADIUS = ENVIRONMENT.settings.streamline_radius;
    DENSITY_MULTIPLIER = PI * RADIUS * RADIUS;

    var offset = 0u;

    while (offset < n_indices) {
        if (local == 0) {
            OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { continue; }

            let index = LINE_INDEX[index_index];
            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            voxelize(index, v0, v1, RADIUS);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex, length: f32) {
    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));
    let density = DENSITY_MULTIPLIER * length;

    atomicAdd(&DENSITY[idx], encode_density(density));
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    let smoothing = ENVIRONMENT.settings.smoothing;

    let radius_clamp = max(smoothing, RADIUS);
    let radius_ratio = pow(RADIUS / radius_clamp, 2.0);

    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));

    let sample = vec3<f32>(voxel) + 0.5;

    let sample_v0 = sample - v0.xyz;
    let delta = v1.xyz - v0.xyz;
    let height = clamp(dot(sample_v0, delta) / dot(delta, delta), 0.0, 1.0);

    // let sdf = cylinder(sample, v0.xyz, v1.xyz, radius_clamp);

    let sdf = capsule(sample, v0, v1, radius_clamp);

    let density = radius_ratio * mix(v0.alpha, v1.alpha, height) * saturate(0.5 - sdf);

    atomicAdd(&DENSITY[idx], encode_density(density));
}

fn encode_density(density: f32) -> u32 {
    return (u32(density * U12_MAX_f32) << U14_SHIFT) + 1;
}

fn capsule(p: vec3<f32>, a: Vertex, b: Vertex, r: f32) -> f32 {
    let ba = b.xyz - a.xyz;
    let pa = p.xyz - a.xyz;
    let pb = p.xyz - b.xyz;

    let h = saturate(dot(pa, ba) / dot(ba, ba));
    let sdf = length(pa - ba * h) - r;

    if (ENVIRONMENT.settings.shadows > 0.5) { return sdf; }

    let plane_a = dot(pa,-a.clip);
    let plane_b = dot(pb, b.clip);

    return max(sdf, max(plane_a, plane_b));
}
