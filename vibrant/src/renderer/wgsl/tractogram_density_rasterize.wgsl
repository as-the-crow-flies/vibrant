// Used instead of vec3<f32> for padding reasons
struct Vertex {
    x: f32,
    y: f32,
    z: f32
}

@group(0) @binding(0) var<storage, read_write> VOLUME:
    array<array<array<atomic<u32>, VOLUME_XYZ>, VOLUME_XYZ>, VOLUME_XYZ>;
@group(0) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;
@group(0) @binding(2) var<uniform> VOLUME_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<Vertex>;
@group(1) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288;
const U32_MAX: u32 = 4294967295;

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, 1.0);
}

@compute
@workgroup_size(WORKGROUP_X)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&TRACTOGRAM_INDICES)) { return; }

    let indices = vec2<u32>(TRACTOGRAM_INDICES[id.x], TRACTOGRAM_INDICES[id.x + 1]);

    if (indices.x == U32_MAX || indices.y == U32_MAX) { return; }

    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;
    let transform_normal = mat3x3<f32>(transform[0].xyz, transform[1].xyz, transform[2].xyz);

    let start = transform * get_vertex(indices.x);
    let end = transform * get_vertex(indices.y);
    let delta = end - start;

    let radius = length(transform_normal * vec3<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0));
    let volume = u32(length(delta) * PI * radius * radius * f32(U32_MAX));

    let voxel = vec3<u32>(.5 * (start.xyz + end.xyz));

    let original = atomicAdd(&VOLUME[voxel.z][voxel.y][voxel.x], volume);

    // Prevent U32 Overflow
    if (original + volume < original) {
        atomicStore(&VOLUME[voxel.z][voxel.y][voxel.x], U32_MAX);
    }
}
