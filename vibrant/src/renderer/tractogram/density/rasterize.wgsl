const DIM: u32 = 256;

@group(0) @binding(0) var<storage, read_write> VOLUME: array<array<array<atomic<u32>, DIM>, DIM>, DIM>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(2) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288;
const U32_MAX: u32 = 4294967295;

@compute
@workgroup_size(1024)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&TRACTOGRAM_INDICES)) { return; }

    let index = TRACTOGRAM_INDICES[id.x];

    // Compute Area, relative to voxel size in range 0..U32_MAX
    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(f32(DIM) * ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));
    let area = PI * radius * radius * f32(U32_MAX);

    let v0 = ((TRACTOGRAM_TO_WORLD * TRACTOGRAM_VERTICES[index + 0u]).xyz + 0.5) * f32(DIM);
    let v1 = ((TRACTOGRAM_TO_WORLD * TRACTOGRAM_VERTICES[index + 1u]).xyz + 0.5) * f32(DIM);

    let delta = v1 - v0;
    let total_distance = length(delta);
    let direction = delta / total_distance;
    let distance_between_voxel_boundaries = 1.0 / abs(direction);

    var distance_left = total_distance;
    var distance_to_next_voxel_boundary = one_if_zero(fract(sign(-direction) * fract(v0.xyz))) * distance_between_voxel_boundaries;

    while (distance_left > 0.0) {
        let increment = min(minimum(distance_to_next_voxel_boundary), distance_left);

        // Contribute Cylinder Volume Fraction to Voxel
        let voxel = vec3<u32>(v1.xyz - distance_left * direction);
        let volume = u32(area * increment);

        let original = atomicAdd(&VOLUME[voxel.z][voxel.y][voxel.x], volume);

        // Prevent U32 Overflow
        if (original + volume < original) {
            atomicStore(&VOLUME[voxel.z][voxel.y][voxel.x], U32_MAX);
        }

        // Update Distances
        distance_to_next_voxel_boundary = select(
            distance_to_next_voxel_boundary - increment,
            distance_between_voxel_boundaries,
            distance_to_next_voxel_boundary == vec3<f32>(increment)
        );

        distance_left -= increment;
    }
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}
