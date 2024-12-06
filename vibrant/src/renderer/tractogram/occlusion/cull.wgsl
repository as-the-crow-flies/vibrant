// Used instead of vec3<f32> for padding reasons
struct Vertex {
    x: f32,
    y: f32,
    z: f32
}

@group(0) @binding(0) var OCCLUSION: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<Vertex>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var<storage, read_write> TRACTOGRAM_INDICES: array<u32>;
@group(3) @binding(1) var<storage, read_write> OFFSET: atomic<u32>;

var<workgroup> WORKGROUP: array<u32, ITEMS_PER_WORKGROUP>;
var<workgroup> WORKGROUP_OFFSET: u32;

const U32_MAX: u32 = 4294967295;
const ITEMS_PER_WORKGROUP: u32 = 2u * WORKGROUP_X;

@compute
@workgroup_size(WORKGROUP_X)
fn compute(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) local: u32) {

    let global = id.x;

    let global_0 = 2u * global + 0u;
    let global_1 = 2u * global + 1u;

    let local_0 = 2u * local + 0u;
    let local_1 = 2u * local + 1u;

    let keep_0 = should_keep_segment(global_0);
    let keep_1 = should_keep_segment(global_1);

    WORKGROUP[local_0] = u32(keep_0);
    WORKGROUP[local_1] = u32(keep_1);
    workgroupBarrier();

    reduce(local);

    if local == 0u {
        // Add workgroup sum to OFFSET buffer & get original index
        WORKGROUP_OFFSET = atomicAdd(&OFFSET, WORKGROUP[ITEMS_PER_WORKGROUP - 1u]);

        // Clear last element
        WORKGROUP[ITEMS_PER_WORKGROUP - 1u] = 0u;
    }

    downsweep(local);

    let index_0 = WORKGROUP_OFFSET + WORKGROUP[local_0];
    let index_1 = WORKGROUP_OFFSET + WORKGROUP[local_1];

    if (keep_0) { TRACTOGRAM_INDICES[index_0] = global_0; }
    if (keep_1) { TRACTOGRAM_INDICES[index_1] = global_1; }
}

fn should_keep_segment(index: u32) -> bool {
    let v0 = get_vertex(index);
    let v1 = get_vertex(index + 1);

    if (v0.w == 0.0 || v1.w == 0.0) { return false; }

    let v0_world = TRACTOGRAM_TO_WORLD * v0;
    let v1_world = TRACTOGRAM_TO_WORLD * v1;

    var v0_screen = ENVIRONMENT.camera.projection * v0_world;
        v0_screen /= v0_screen.w;
    var v1_screen = ENVIRONMENT.camera.projection * v1_world;
        v1_screen /= v1_screen.w;

    let bounds_min = vec2<f32>(-1.0);
    let bounds_max = vec2<f32>( 1.0);

    if (
        v0_screen.z <= 0.0 ||
        v0_screen.z >= 1.0 ||
        v1_screen.z <= 0.0 ||
        v1_screen.z >= 1.0 ||
        (
            (any(v0_screen.xy <= bounds_min) || any(v0_screen.xy >= bounds_max)) &&
            (any(v1_screen.xy <= bounds_min) || any(v1_screen.xy >= bounds_max)))
        )
    {
        return false;
    }

    let vmin = min(v0_world.xyz, v1_world.xyz);
    let vmax = max(v0_world.xyz, v1_world.xyz);
    let vdim = vmax - vmin;

    let centroid = 0.5 * (vmin + vmax);
    let level = max(0.0, log2(max(max(vdim.x, vdim.y), vdim.z) * f32(VOLUME_XYZ)));

    return textureSampleLevel(OCCLUSION, SAMPLER, centroid + 0.5, level).x < ENVIRONMENT.settings.cull_level;
}

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, f32(v.x < 1E9));
}

fn reduce(local: u32) {
    var offset = 1u;
    for (var d = ITEMS_PER_WORKGROUP >> 1u; d > 0u; d >>= 1u) {

        if local < d {
            let ai = offset * (2u * local + 1u) - 1u;
            let bi = offset * (2u * local + 2u) - 1u;

            WORKGROUP[bi] += WORKGROUP[ai];
        }

        offset <<= 1u;

        workgroupBarrier();
    }
}

fn downsweep(local: u32) {
    var offset = ITEMS_PER_WORKGROUP;
    for (var d = 1u; d < ITEMS_PER_WORKGROUP; d <<= 1u) {
        offset >>= 1u;

        if local < d {
            let ai = offset * (2u * local + 1u) - 1u;
            let bi = offset * (2u * local + 2u) - 1u;

            let tmp = WORKGROUP[ai];
            WORKGROUP[ai] = WORKGROUP[bi];
            WORKGROUP[bi] += tmp;
        }

        workgroupBarrier();
    }
}
