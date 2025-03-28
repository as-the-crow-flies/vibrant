@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

const PI: f32 = 3.14159265358979323846264338327950288;
const U16_MAX: u32 = 65535;

@compute
@workgroup_size(1024)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(subgroup_invocation_id) subgroup_id: u32) {
    let indices_length = arrayLength(&TRACTOGRAM_INDICES);
    let index = id.x;

    if (index >= indices_length) { return; }

    let v0_index = TRACTOGRAM_INDICES[index];
    let v1_index = v0_index + 1;

    // Compute Area, relative to voxel size in range 0..U16_MAX
    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(f32(ENVIRONMENT.volume) * ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));
    let area = PI * radius * radius * f32(U16_MAX);

    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;

    let v0 = (transform * TRACTOGRAM_VERTICES[v0_index]).xyz;
    let v1 = (transform * TRACTOGRAM_VERTICES[v1_index]).xyz;

    let delta = v1 - v0;
    let total_distance = length(delta);
    let direction = delta / total_distance;
    let voxel_boundaries = 1.0 / abs(direction);

    let step = vec3<i32>(sign(direction));

    var next = vec4<f32>(
        one_if_zero(abs(fract(vec3<f32>(-step) * fract(v0)))) * voxel_boundaries,
        total_distance
    );

    var voxel = vec3<i32>(v0);

    while (next.w > 0.0) {
        let increment = minimum(next);

        let density = u32(area * increment);
        let idx = linear_index(vec3<u32>(voxel));

        atomicAdd(&DENSITY[idx], density);

        let mask = next == vec4<f32>(increment);
        voxel += select(vec3<i32>(0), step, mask.xyz);
        next = select(
            next - increment,
            vec4<f32>(voxel_boundaries, 0.0),
            mask
        );
    }
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}
