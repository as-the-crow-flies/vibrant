@group(2) @binding(0) var<storage, read_write> VERTICES: array<u32>;

@group(3) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

struct Hit {
    distance: f32,
    voxel: vec3<u32>,
    index: u32
}

var<private> HIT: Hit = Hit(0.0, vec3<u32>(), U32_MAX);

fn visit(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) -> bool {

    HIT = Hit(distance, vec3<u32>(), U32_MAX);

    let start = textureLoad(START, voxel).x;
    let end = textureLoad(END, voxel).x;

    for (var index = start; index < end; index++) {
        let segment = decode_segment(voxel, VERTICES[index]);

        let v0 = segment.v0 * DIM_INV - 0.5;
        let v1 = segment.v1 * DIM_INV - 0.5;

        let intersection = capsule_intersection(origin, direction, v0, v1, RADIUS);

        if (intersection < HIT.distance) {
            HIT = Hit(intersection, voxel, index);
        }
    }

    return HIT.index != U32_MAX;
}

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    if (HIT.index == U32_MAX) { return vec4<f32>(0.0); }

    let segment = decode_segment(HIT.voxel, VERTICES[HIT.index]);

    let v0 = Vertex(segment.v0 * DIM_INV - 0.5, vec3<f32>(), 1.0);
    let v1 = Vertex(segment.v1 * DIM_INV - 0.5, vec3<f32>(), 1.0);

    let position = origin + direction * HIT.distance;

    return shade(v0, v1, RADIUS, position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);
}
