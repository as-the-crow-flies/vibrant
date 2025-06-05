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
    let n_indices = arrayLength(&TRACTOGRAM_INDICES);

    loop {
        if (local == 0) {
            OFFSET = atomicAdd(&TRACTOGRAM_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        let offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;
            if (index_index > n_indices) { return; }

            let index = TRACTOGRAM_INDICES[index_index];

            let v = TRACTOGRAM_VERTICES[index    ].xyz;
            let vp = TRACTOGRAM_VERTICES[index + 1].xyz;
            let vm = TRACTOGRAM_VERTICES[index - 1].xyz;

            let endpoint = any(vp > vec3<f32>(1E6)) || any(vm > vec3<f32>(1E6));
            let plane = select(normalize(vp - vm), vec3<f32>(0), endpoint);

            TRACTOGRAM_VERTICES[index].w = bitcast<f32>(pack4x8snorm(vec4<f32>(plane, 0.0)));
        }
    }
}
