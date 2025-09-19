@group(2) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(2) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;

@group(3) @binding(0) var<storage> OFFSET: array<u32>;
@group(3) @binding(2) var<storage> INDEX: array<u32>;

struct Hit {
    distance: f32,
    index: u32
}

var<private> HIT: Hit = Hit(0.0, U32_MAX);

fn visit(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32,
    axis: u32) -> bool {

    let count = textureLoad(COUNT, voxel, 0).x;

    if (count == 0) { return false; }

    let offset = OFFSET[block_index(voxel, textureDimensions(DENSITY))] - count;

    HIT = Hit(distance, U32_MAX);

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let v0 = LINE_VERTEX[index + 0];
        let v1 = LINE_VERTEX[index + 1];

        let intersection = capsule_intersection(origin, direction, v0.xyz, v1.xyz, RADIUS);

        if (intersection < HIT.distance) {
            HIT = Hit(intersection, index);
        }
    }

    return HIT.index != U32_MAX;
}

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    if (HIT.index == U32_MAX) { return vec4<f32>(0.0); }

    let v0 = unpack_vertex(LINE_VERTEX[HIT.index + 0]);
    let v1 = unpack_vertex(LINE_VERTEX[HIT.index + 1]);

    let position = origin + direction * HIT.distance;

    return shade(v0, v1, RADIUS, position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);
}
