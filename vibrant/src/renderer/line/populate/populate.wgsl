@group(0) @binding(0) var<storage, read_write> OFFSET: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> INDEX: array<u32>;

@group(1) @binding(0) var CULLING: texture_3d<f32>;

@group(2) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(2) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(2) @binding(2) var<storage> LINE_LENGTH: u32;
@group(2) @binding(3) var<storage, read_write> LINE_OFFSET: atomic<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 256;
const CHUNK_SIZE: u32 = 32;

var<workgroup> WORKGROUP_OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = LINE_LENGTH;
    let scale = f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius;

    var offset = 0u;

    while (offset < n_indices) {
        if (local == 0) {
            WORKGROUP_OFFSET = atomicAdd(&LINE_OFFSET, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&WORKGROUP_OFFSET);

        // Voxelize every segment unconditionally.  The CULLING filter previously
        // skipped occluded segments to save INDEX space, but that made the index
        // camera-dependent.  Removing it lets the index be cached across frames
        // whenever only the camera changes.
        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { continue; }

            let index = LINE_INDEX[index_index];

            let v0 = unpack_vertex_scale(LINE_VERTEX[index + 0], scale);
            let v1 = unpack_vertex_scale(LINE_VERTEX[index + 1], scale);

            voxelize(index, v0, v1, radius);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex, length: f32) {
    let idx = block_index(vec3<u32>(voxel), textureDimensions(CULLING));
    INDEX[atomicAdd(&OFFSET[idx], 1u)] = index;
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    visit_voxel_line(voxel, index, v0, v1, 0.0);
}
