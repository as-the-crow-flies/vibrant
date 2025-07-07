@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(2) var<storage, read_write> TRACTOGRAM_VERTICES: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> TRACTOGRAM_INDICES: array<u32>;
@group(0) @binding(4) var<storage, read_write> TRACTOGRAM_COUNT: atomic<u32>;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_vertices = arrayLength(&TRACTOGRAM_VERTICES);

    loop {
        if (local == 0) {
            OFFSET = atomicAdd(&TRACTOGRAM_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        let offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index = offset + i * WORKGROUP_SIZE + local;
            if (index >= n_vertices) { return; }

            let vertex = TRACTOGRAM_VERTICES[index];

            TRACTOGRAM_VERTICES[index] = vec4<f32>(transform(TRACTOGRAM_TO_WORLD, vertex), 1.0);
        }
    }
}
