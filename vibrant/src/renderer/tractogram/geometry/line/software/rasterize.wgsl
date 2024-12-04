// Used instead of vec3<f32> for padding reasons
struct Vertex {
    x: f32,
    y: f32,
    z: f32
}

@group(0) @binding(0) var<storage, read_write> KBUFFER: array<array<atomic<u64>, SURFACE_X>, SURFACE_Y>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<Vertex>;

@group(2) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const U32_MAX: u32 = 4294967295;

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, 1.0);
}

@compute
@workgroup_size(WORKGROUP_X)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&TRACTOGRAM_INDICES)) { return; }

    let index = TRACTOGRAM_INDICES[id.x];

    let v0 = TRACTOGRAM_TO_WORLD * get_vertex(index);
    let v1 = TRACTOGRAM_TO_WORLD * get_vertex(index + 1);

    let tangent = abs(normalize(v1.xyz - v0.xyz));

    let start = transform(v0);
    let end = transform(v1);

    let delta = end - start;

    let total_distance = min(length(delta.xy), 100.0); // Safety Feature :D
    let direction = delta / total_distance;
    let distance_between_voxel_boundaries = 1.0 / abs(direction.xy);

    var distance_left = total_distance;
    var distance_to_next_voxel_boundary = one_if_zero(fract(sign(-direction.xy) * fract(start.xy))) * distance_between_voxel_boundaries;

    while (distance_left > 0.0) {
        let increment = min(minimum(distance_to_next_voxel_boundary), distance_left);
        let alpha = min(length(increment * direction), 1.0);

        let sample = end.xyz - distance_left * direction;

        let depth = u64(sample.z * f32(U32_MAX));
        let payload = u64(pack4x8unorm(vec4<f32>(tangent, alpha)));
        let visibility = depth << 32u | payload;

        atomicMin(&KBUFFER[u32(sample.y)][u32(sample.x)], visibility);

        // Update Distances
        distance_to_next_voxel_boundary = select(
            distance_to_next_voxel_boundary - increment,
            distance_between_voxel_boundaries,
            distance_to_next_voxel_boundary == vec2<f32>(increment)
        );

        distance_left -= increment;
    }
}

fn transform(vertex: vec4<f32>) -> vec3<f32> {
    let v = ENVIRONMENT.camera.projection * vertex;
    let vt = v.xyz / v.w * 0.5 + 0.5;
    return vec3<f32>(vt.x * f32(SURFACE_X), (1.0 - vt.y) * f32(SURFACE_Y), vt.z);
}

fn one_if_zero(v: vec2<f32>) -> vec2<f32> {
    return v + vec2<f32>(v == vec2<f32>(0.0));
}

fn minimum(v: vec2<f32>) -> f32 {
    return min(v.x, v.y);
}
