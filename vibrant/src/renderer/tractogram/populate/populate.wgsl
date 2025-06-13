@group(0) @binding(0) var<storage, read_write> OFFSET: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> INDEX: array<u32>;

@group(1) @binding(0) var OCCUPANCY: texture_3d<f32>;
@group(1) @binding(2) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;

@group(2) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(2) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(2) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;
@group(2) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(4) var<storage, read_write> TRACTOGRAM_COUNT: atomic<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

var<workgroup> WORKGROUP_OFFSET: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&TRACTOGRAM_INDICES);
    let radius = ENVIRONMENT.settings.streamline_radius;
    let TRANSFORM = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;

    loop {
        if (local == 0) {
            WORKGROUP_OFFSET = atomicAdd(&TRACTOGRAM_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        let offset = workgroupUniformLoad(&WORKGROUP_OFFSET);

        var i = 0u;

        loop {
            var index = 0u;
            var v0 = vec3<f32>();
            var v1 = vec3<f32>();

            loop {
                if (i >= CHUNK_SIZE) { return; }

                let index_index = offset + i * WORKGROUP_SIZE + local;

                if (index_index > n_indices) { return; }

                index = TRACTOGRAM_INDICES[index_index];
                v0 = transform(TRANSFORM, TRACTOGRAM_VERTICES[index + 0]);
                v1 = transform(TRANSFORM, TRACTOGRAM_VERTICES[index + 1]);

                i++;

                if (occupancy(v0, v1) > 0.0) { break; }
            }

            voxelize(index, v0, v1, radius);
        }
    }
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: vec3<f32>, v1: vec3<f32>) {
    let should_write = textureLoad(OCCUPANCY, voxel, 0).x > 0.0;

    if (should_write) {
        let offset = &OFFSET[block_index(vec3<u32>(voxel), textureDimensions(OCCUPANCY))];
        INDEX[atomicAdd(offset, 1u)] = index;
    }
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
