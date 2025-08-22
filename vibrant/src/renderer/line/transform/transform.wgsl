@group(0) @binding(0) var<storage> LINE_INDEX_RAW: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX_RAW: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> TRANSFORM: mat4x4<f32>;

@group(1) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> LINE_LENGTH: u32;
@group(1) @binding(3) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_vertices = arrayLength(&LINE_VERTEX);

    var offset = 0u;

    while (offset < n_vertices) {
        if (local == 0) {
            OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = offset + i * WORKGROUP_SIZE + local;

            if (index >= n_vertices) { continue; }

            let vertex = LINE_VERTEX_RAW[index];

            LINE_VERTEX[index] = vec4<f32>(
                (TRANSFORM * vec4<f32>(vertex.xyz, 1.0)).xzy,
                pack_clip_alpha(vec4<f32>(0.0, 0.0, 0.0, vertex.a))
            );
        }
    }
}
