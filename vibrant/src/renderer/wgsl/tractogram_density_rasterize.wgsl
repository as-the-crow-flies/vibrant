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

const INFINITY: f32 = 0x7F7FFFFF;
const PI: f32 = 3.14159265358979323846264338327950288;
const U32_MAX: u32 = 4294967295;
const EPSILON: f32 = 1E-9;

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, 1.0);
}

@compute
@workgroup_size(WORKGROUP_X)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;
    let transform_normal = mat3x3<f32>(transform[0].xyz, transform[1].xyz, transform[2].xyz);

    let start = (transform * get_vertex(id.x)).xyz;
    let end = (transform * get_vertex(id.x + 1)).xyz;

    if (start.x == INFINITY || end.x == INFINITY) { return; }

    let delta = end - start;

    let radius = length(transform_normal * vec3<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0));
    let area = PI * radius * radius;

    let direction = normalize(delta);
    let direction_abs_inv = 1.0 / max(abs(direction), vec3<f32>(EPSILON));

    // length of largest dimension
    let delta_length = maximum(abs(delta));

    // Target Voxel Boundary, Depending on the sign: 0.0 if negative, 1.0 if positive
    let delta_target = max(vec3<f32>(0.0), sign(delta));

    var distance = 0.0;
    var sample = start.xyz;
    var i = 0u;

    while (distance < delta_length && i < 10u) {
        // Distance from current sample to next voxel boundaries, traveling along line direction
        let difference = abs(delta_target - fract(sample));
        let one_if_zero = vec3<f32>(difference == vec3<f32>(0.0));
        let distance_to_closest_intersection = minimum((difference + one_if_zero) * direction_abs_inv);

        // First voxel boundary that will be hit
        let step = min(distance_to_closest_intersection, delta_length - distance);

        // Contribute Cylinder Volume Fraction to Voxel
        let volume = u32(area * step * f32(U32_MAX));
        let voxel = vec3<u32>(sample);
        let original = atomicAdd(&VOLUME[voxel.z][voxel.y][voxel.x], volume);

        // Prevent U32 Overflow
        if (original + volume < original) {
            atomicStore(&VOLUME[voxel.z][voxel.y][voxel.x], U32_MAX);
        }

        // Advance to the next closest voxel boundary
        distance += step;
        sample += direction * step;
        i++;
    }
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}
