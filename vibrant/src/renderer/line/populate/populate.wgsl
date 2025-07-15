@group(0) @binding(0) var<storage, read_write> OFFSET: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> INDEX: array<u32>;

@group(1) @binding(0) var OCCUPANCY: texture_3d<f32>;

@group(2) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(2) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(2) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> WORKGROUP_OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&LINE_INDEX);
    let radius = ENVIRONMENT.settings.streamline_radius;

    loop {
        if (local == 0) {
            WORKGROUP_OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        let offset = workgroupUniformLoad(&WORKGROUP_OFFSET);

        var i = 0u;

        loop {
            var index = 0u;
            var v0 = Vertex();
            var v1 = Vertex();

            loop {
                if (i >= CHUNK_SIZE) { return; }

                let index_index = offset + i * WORKGROUP_SIZE + local;

                if (index_index >= n_indices) { return; }

                index = LINE_INDEX[index_index];
                v0 = unpack_vertex(LINE_VERTEX[index + 0]);
                v1 = unpack_vertex(LINE_VERTEX[index + 1]);

                i++;

                if (occupancy(v0.xyz, v1.xyz) > 0.0) { break; }
            }

            voxelize(index, v0, v1, radius);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, index: u32, length: f32) {
    let should_write = textureLoad(OCCUPANCY, voxel, 0).x > 0.0;

    if (should_write) {
        let idx = block_index(vec3<u32>(voxel), textureDimensions(OCCUPANCY));
        let offset = &OFFSET[idx];
        INDEX[atomicAdd(offset, 1u)] = index;
    }
}

fn visit_voxel_ground_truth(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    visit_voxel_line(voxel, index, 0.0);
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    visit_voxel_line(voxel, index, 0.0);
}

fn occupancy(v0: vec3<f32>, v1: vec3<f32>) -> f32 {
    let level = maximum(32 - countLeadingZeros(vec3<u32>(v0) ^ vec3<u32>(v1)));

    return select(
        1.0,
        textureLoad(OCCUPANCY, vec3<u32>(v0) >> vec3<u32>(level), i32(level)).x,
        level < textureNumLevels(OCCUPANCY)
    );
}

fn maximum(v: vec3<u32>) -> u32 {
    return max(max(v.x, v.y), v.z);
}
