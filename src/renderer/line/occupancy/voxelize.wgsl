use block;
use depth;
use math;
use uniforms;
use vertex;

@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;

@group(1) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage> LINE_LENGTH: u32;
@group(1) @binding(3) var<storage, read_write> LINE_OFFSET: atomic<u32>;

@group(2) @binding(0) var<uniform> TRACTOGRAPHY: Tractography;

const WORKGROUP_SIZE: u32 = 256;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

var<private> RADIUS: f32;
var<private> DENSITY_MULTIPLIER: f32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = LINE_LENGTH;
    let scale = f32(TRACTOGRAPHY.volume);

    RADIUS = TRACTOGRAPHY.radius;
    DENSITY_MULTIPLIER = PI * RADIUS * RADIUS;

    var offset = 0u;

    while (offset < n_indices) {
        if (local == 0) {
            OFFSET = atomicAdd(&LINE_OFFSET, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { continue; }

            let index = LINE_INDEX[index_index];

            let v0 = unpack_vertex_scale(LINE_VERTEX[index + 0], scale);
            let v1 = unpack_vertex_scale(LINE_VERTEX[index + 1], scale);

            voxelize(index, v0, v1, RADIUS);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex, length: f32) {
    let dim = vec3<u32>(TRACTOGRAPHY.volume);

    if (!in_volume(voxel, dim)) { return; }

    accumulate(block_index(vec3<u32>(voxel), dim), index, DENSITY_MULTIPLIER * length);
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    let dim = vec3<u32>(TRACTOGRAPHY.volume);

    if (!in_volume(voxel, dim)) { return; }

    let smoothing = TRACTOGRAPHY.smoothing;

    let radius_clamp = max(smoothing, RADIUS);
    let radius_ratio = pow(RADIUS / radius_clamp, 2.0);

    let idx = block_index(vec3<u32>(voxel), dim);

    let p = vec3<f32>(voxel) + 0.5;

    let delta = v1.xyz - v0.xyz;
    let pv0 = p.xyz - v0.xyz;
    let pv1 = p.xyz - v1.xyz;

    let height = saturate(dot(pv0, delta) / dot(delta, delta));

    let sdf = max(length(pv0 - delta * height) - radius_clamp, max(-dot(pv0, v0.clip), dot(pv1, v1.clip)));

    let coverage = radius_ratio * mix(v0.alpha, v1.alpha, height) * saturate(0.5 - sdf);

    accumulate(idx, index, coverage);
}

fn accumulate(idx: u32, index: u32, coverage: f32) {
    if (atomicLoad(&DENSITY[idx]) >= DEPTH_SATURATED) {
        atomicAdd(&DENSITY[idx], 1u);
        return;
    }

    atomicAdd(&DENSITY[idx], encode_depth(coverage, dither(idx, index)) + 1u);
}
