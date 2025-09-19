@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage> LINE_LENGTH: u32;
@group(0) @binding(3) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(1) @binding(0) var<storage, read_write> KEY: array<u32>;
@group(1) @binding(1) var<storage, read_write> VALUE: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&LINE_INDEX);

    let eye = ENVIRONMENT.camera.transform[3].xyz;
    let near = ENVIRONMENT.camera.near;
    let far_minus_near_inv = 1.0 / (ENVIRONMENT.camera.far - ENVIRONMENT.camera.near);

    let scale = 1.0 / f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius * scale;

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
            let v0 = LINE_VERTEX[index + 0].xyz;
            let v1 = LINE_VERTEX[index + 1].xyz;

            let sdf = capsule_sdf(eye, v0, v1, radius);

            KEY[index_index] = u32(saturate((sdf - near) * far_minus_near_inv) * U32_MAX_f32);
            VALUE[index_index] = index;
        }
    }
}
