// Used instead of vec3<f32> for padding reasons
struct Vertex {
    x: f32,
    y: f32,
    z: f32
}

@group(0) @binding(0) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(0) @binding(1) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<Vertex>;
@group(0) @binding(3) var<storage, read_write> TRACTOGRAM_INDICES: array<u32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(2) @binding(0) var<storage, read_write> VISIBILITY: array<array<atomic<u32>, SURFACE_X>, SURFACE_Y>;

@group(3) @binding(0) var<storage, read_write> INDIRECT: atomic<u32>;

const U32_MAX: u32 = 4294967295;
const U16_MAX: u32 = 65535;
const ONE_OVER_U32_MAX: f32 = 0.000000000232831;

@compute
@workgroup_size(WORKGROUP_XY, WORKGROUP_XY)
fn clear(@builtin(global_invocation_id) id: vec3<u32>) {
    atomicStore(&VISIBILITY[id.y][id.x], U32_MAX);
}

@compute
@workgroup_size(WORKGROUP_X)
fn cull(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= arrayLength(&TRACTOGRAM_VERTICES)) { return; }

    let start_vertex = get_vertex(id.x);
    let end_vertex = get_vertex(id.x + 1);

    if (start_vertex.w == 0 || end_vertex.w == 0) { return; }

    let start_vertex_world_space = TRACTOGRAM_TO_WORLD * start_vertex;
    let end_vertex_world_space = TRACTOGRAM_TO_WORLD * end_vertex;

    let start = transform(start_vertex_world_space);
    let end = transform(end_vertex_world_space);

    let delta = end - start;

    let bounds_min = vec3<f32>(0.0);
    let bounds_max = vec3<f32>(f32(SURFACE_X), f32(SURFACE_Y), 1.0);

    if (
        all(start >= bounds_min) &&
        all(end >= bounds_min) &&
        all(start < bounds_max) &&
        all(end < bounds_max))
    {
        TRACTOGRAM_INDICES[atomicAdd(&INDIRECT, 1u)] = id.x;
    }
}

@compute
@workgroup_size(1)
fn set_dispatch_count() {
    atomicStore(&INDIRECT, div_ceil(atomicLoad(&INDIRECT), WORKGROUP_X));
}

@compute
@workgroup_size(WORKGROUP_X)
fn rasterize(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = TRACTOGRAM_INDICES[id.x];
    let start_vertex = get_vertex(index);
    let end_vertex = get_vertex(index + 1u);

    let start_vertex_world_space = TRACTOGRAM_TO_WORLD * start_vertex;
    let end_vertex_world_space = TRACTOGRAM_TO_WORLD * end_vertex;

    let tangent_world_space = abs(normalize(end_vertex_world_space - start_vertex_world_space));
    let tangent_world_space_bits = pack4x8unorm(vec4<f32>(tangent_world_space.x, tangent_world_space.y, 0.0, 0.0));

    let start = transform(start_vertex_world_space);
    let end = transform(end_vertex_world_space);

    let delta = end - start;

    let max_distance = min(u32(max(abs(delta.x), abs(delta.y))), 100u); // Safety Measure
    let step = delta / f32(max_distance);

    var sample = start;
    for (var distance = 0u; distance < max_distance; distance++) {
        let depth_bits = pack2x16unorm(vec2<f32>(0.0, sample.z));
        let visibility = depth_bits | tangent_world_space_bits;

        atomicMin(&VISIBILITY[u32(sample.y)][u32(sample.x)], visibility);

        sample += step;
    }
}

@fragment
fn shade(@builtin(position) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let visibility = atomicLoad(&VISIBILITY[u32(uv.y)][u32(uv.x)]);
    let depth = unpack2x16unorm(visibility).y;
    let tangent_xy = unpack4x8unorm(visibility).xy;

    if (depth >= 1.0) { discard; }

    let tangent_z = sqrt(1.0 - tangent_xy.x * tangent_xy.x - tangent_xy.y * tangent_xy.y);
    let tangent = vec3<f32>(tangent_xy, tangent_z);

    return vec4<f32>(tangent, 1.0);
}

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, f32(v.x < 1E9));
}

fn transform(vertex: vec4<f32>) -> vec3<f32> {
    let v = ENVIRONMENT.camera.projection * vertex;
    let vt = v.xyz / v.w * 0.5 + 0.5;
    return vec3<f32>(vt.x * f32(SURFACE_X), (1.0 - vt.y) * f32(SURFACE_Y), vt.z);
}

fn div_ceil(a: u32, b: u32) -> u32 {
    return (a + b - 1u) / b;
}
