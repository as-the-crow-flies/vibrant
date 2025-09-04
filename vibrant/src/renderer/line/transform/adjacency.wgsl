@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(2) var<storage> LINE_LENGTH: u32;
@group(0) @binding(3) var<storage, read_write> LINE_COUNT: atomic<u32>;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&LINE_INDEX);

    var offset = 0u;

    while (offset < n_indices) {
        if (local == 0) {
            OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { continue; }

            let vmi = LINE_INDEX[index_index - 1];
            let vi  = LINE_INDEX[index_index + 0];
            let vpi = LINE_INDEX[index_index + 1];

            let endpoint = abs(vi - vmi) != 1;

            let vm = LINE_VERTEX[vi - 1].xyz;
            let v  = LINE_VERTEX[vi ];
            let vp = LINE_VERTEX[vi + 1].xyz;

            let clip = select(normalize(vp - vm), vec3<f32>(0), endpoint);

            let alpha = unpack_clip_alpha(v.a).a;

            LINE_VERTEX[vi] = vec4<f32>(v.xyz, pack_clip_alpha(vec4<f32>(clip, alpha)));
        }
    }
}
