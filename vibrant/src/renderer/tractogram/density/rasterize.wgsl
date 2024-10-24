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

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.14159265358979323846264338327950288;
const U32_MAX: u32 = 4294967295;

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, f32(v.x < 1E9));
}

@compute
@workgroup_size(WORKGROUP_X)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let start_vertex = get_vertex(id.x);
    let end_vertex = get_vertex(id.x + 1);

    if (start_vertex.w == 0 || end_vertex.w == 0) { return; }

    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;

    // Compute Area, relative to voxel size in range 0..U32_MAX
    let radius = length(transform * vec4<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));
    let area = PI * radius * radius * f32(U32_MAX);

    let start = (transform * start_vertex).xyz;
    let end = (transform * end_vertex).xyz;
    let delta = end - start;
    let total_distance = length(delta);
    let direction = delta / total_distance;
    let distance_between_voxel_boundaries = 1.0 / abs(direction);

    var distance_left = total_distance;
    var distance_to_next_voxel_boundary = one_if_zero(fract(sign(-direction) * fract(start.xyz))) * distance_between_voxel_boundaries;

    while (distance_left > 0.0) {
        let increment = min(minimum(distance_to_next_voxel_boundary), distance_left);

        // Contribute Cylinder Volume Fraction to Voxel
        let voxel = vec3<u32>(end.xyz - distance_left * direction);
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
