@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage> LINE_INDICES_LENGTH: u32;
@group(0) @binding(3) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(1) @binding(0) var<storage, read_write> KEY: array<u32>;
@group(1) @binding(1) var<storage, read_write> VALUE: array<u32>;
@group(1) @binding(2) var<storage, read_write> COUNT: atomic<u32>;

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
    let n_indices = LINE_INDICES_LENGTH;
    let scale = f32(ENVIRONMENT.volume);

    RADIUS = ENVIRONMENT.settings.radius;
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
            let v0 = unpack_vertex_scale(LINE_VERTEX[index + 0], scale);
            let v1 = unpack_vertex_scale(LINE_VERTEX[index + 1], scale);

            voxelize(index, v0, v1, RADIUS);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, instance_index: u32, v0: Vertex, v1: Vertex, length: f32) {
    let fragment_index = atomicAdd(&COUNT, 1u);

    KEY[fragment_index] = instance_index;
    VALUE[fragment_index] = morton_encode(vec3<u32>(voxel));
}

fn visit_voxel(voxel: vec3<i32>, instance_index: u32, v0: Vertex, v1: Vertex) {
    let fragment_index = atomicAdd(&COUNT, 1u);

    KEY[fragment_index] = instance_index;
    VALUE[fragment_index] = morton_encode(vec3<u32>(voxel));
}
