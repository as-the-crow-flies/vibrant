@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

const PI: f32 = 3.14159265358979323846264338327950288;
const U16_MAX: u32 = 65535;

const WORKGROUP_SIZE: u32 = 1024;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = min(id.x, arrayLength(&TRACTOGRAM_INDICES) - 2);

    let tractogram_index = TRACTOGRAM_INDICES[index];

    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;

    let v0 = (transform * TRACTOGRAM_VERTICES[tractogram_index    ]).xyz;
    let v1 = (transform * TRACTOGRAM_VERTICES[tractogram_index + 1]).xyz;

    voxelize(v0, v1);
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}

fn voxelize(v0: vec3<f32>, v1: vec3<f32>) {
    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(f32(ENVIRONMENT.volume) * ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));
    let area = PI * radius * radius * f32(U16_MAX);

    let delta = v1 - v0;
    let distance = length(delta);
    let direction = delta / distance;
    let voxel_boundaries = 1.0 / abs(direction);
    let step = vec3<i32>(sign(direction));
    var next = vec4<f32>(
        one_if_zero(abs(fract(vec3<f32>(-step) * fract(v0)))) * voxel_boundaries,
        distance
    );

    var voxel = vec3<i32>(v0);

    while (next.w > 0.0) {
        let increment = minimum(next);

        let idx = linear_index(vec3<u32>(voxel));

        atomicAdd(&DENSITY[idx], u32(area * increment));

        let mask = next == vec4<f32>(increment);
        voxel += select(vec3<i32>(0), step, mask.xyz);
        next = select(
            next - increment,
            vec4<f32>(voxel_boundaries, 0.0),
            mask
        );
    }
}
