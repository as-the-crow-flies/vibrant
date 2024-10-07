@group(0) @binding(0) var<storage, read_write> VOLUME:
    array<array<array<atomic<u32>, VOLUME_XYZ>, VOLUME_XYZ>, VOLUME_XYZ>;
@group(0) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;
@group(0) @binding(2) var<uniform> VOLUME_TO_WORLD: mat4x4<f32>;

@group(1) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec3<f32>>;
@group(1) @binding(3) var<storage> TRACTOGRAM_INDICES: array<u32>;

const PI: f32 = 3.14159265358979323846264338327950288;
const U32_MAX: u32 = 4294967295;

@compute
@workgroup_size(WORKGROUP_X)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&TRACTOGRAM_INDICES)) { return; }

    let indices = vec2<u32>(TRACTOGRAM_INDICES[id.x], TRACTOGRAM_INDICES[id.x + 1]);

    if (indices.x == U32_MAX || indices.y == U32_MAX) { return; }

    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;
    let transform_normal = mat3x3<f32>(transform[0].xyz, transform[1].xyz, transform[2].xyz);

    let start = transform * vec4<f32>(TRACTOGRAM_VERTICES[indices.x], 1.0);
    let end = transform * vec4<f32>(TRACTOGRAM_VERTICES[indices.y], 1.0);
    let delta = end - start;

    let radius = length(transform_normal * vec3<f32>(0.01, 0.0, 0.0)); // Assuming Scaling is Uniform
    let volume = u32(length(delta) * PI * radius * radius * f32(U32_MAX));

    let voxel = vec3<u32>(.5 * (start.xyz + end.xyz));

    let original = atomicAdd(&VOLUME[voxel.z][voxel.y][voxel.x], volume);

    // Prevent U32 Overflow
    if (original + volume < original) {
        atomicStore(&VOLUME[voxel.z][voxel.y][voxel.x], U32_MAX);
    }
}
