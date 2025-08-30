@group(0) @binding(0) var<storage, read_write> OFFSET: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> INDEX: array<u32>;

@group(1) @binding(0) var CULLING: texture_3d<f32>;

@group(2) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(2) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(2) @binding(2) var<storage> LINE_INDICES_LENGTH: u32;
@group(2) @binding(3) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> WORKGROUP_OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = LINE_INDICES_LENGTH;
    let scale = f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius;

    var offset = 0u;

    while (offset < n_indices) {
        if (local == 0) {
            WORKGROUP_OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&WORKGROUP_OFFSET);

        var i = 0u;

        while (i < CHUNK_SIZE) {
            var index = 0u;
            var v0 = Vertex();
            var v1 = Vertex();

            while (i < CHUNK_SIZE) {
                let index_index = offset + i * WORKGROUP_SIZE + local;
                i++;

                if (index_index >= n_indices) { continue; }

                index = LINE_INDEX[index_index];
                v0 = unpack_vertex_scale(LINE_VERTEX[index + 0], scale);
                v1 = unpack_vertex_scale(LINE_VERTEX[index + 1], scale);

                if (culling(v0.xyz, v1.xyz) > 0.0) { break; }
            }

            voxelize(index, v0, v1, radius);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex, length: f32) {
    let should_write = textureLoad(CULLING, voxel, 0).x > 0.5;

    if (should_write) {
        let idx = block_index(vec3<u32>(voxel), textureDimensions(CULLING));
        INDEX[atomicAdd(&OFFSET[idx], 1u)] = index;
    }
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    visit_voxel_line(voxel, index, v0, v1, 0.0);
}

fn culling(v0: vec3<f32>, v1: vec3<f32>) -> f32 {
    let v_min = vec3<u32>(min(v0, v1) - ENVIRONMENT.settings.radius);
    let v_max = vec3<u32>(max(v0, v1) + ENVIRONMENT.settings.radius);

    let level = 32 - minimum3(countLeadingZeros(v_min ^ v_max));

    return select(
        1.0,
        textureLoad(CULLING, v_min >> vec3<u32>(level), i32(level)).x,
        level < textureNumLevels(CULLING)
    );
}

fn minimum3(v: vec3<u32>) -> u32 {
    return min(min(v.x, v.y), v.z);
}
