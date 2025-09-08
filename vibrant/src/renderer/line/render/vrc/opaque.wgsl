@group(2) @binding(0) var<storage, read_write> VERTICES: array<u32>;

@group(3) @binding(0) var START: texture_storage_3d<r32uint, read_write>;
@group(3) @binding(1) var END: texture_storage_3d<r32uint, read_write>;

struct Hit {
    distance: f32,
    voxel: vec3<u32>,
    index: u32
}

var<private> HIT: Hit = Hit(1000.0, vec3<u32>(), U32_MAX);

fn visit(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) -> bool {

    let done = HIT.index != U32_MAX;

    if (ENVIRONMENT.settings.shadows > 0.5) {
        for (var x = -1; x <= 1; x++) {
            for (var y = -1; y <= 1; y++) {
                for (var z = -1; z <= 1; z++) {
                    let vxl = vec3<u32>(vec3<i32>(voxel) + vec3<i32>(x, y, z));
                    visit_voxel(vxl, origin, direction, position, increment, distance);
                }
            }
        }
    } else {
        visit_voxel(voxel, origin, direction, position, increment, distance);
    }

    return done;
}

fn visit_voxel(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) {

    let start = textureLoad(START, voxel).x;
    let end = textureLoad(END, voxel).x;

    if (end - start > 128) { return; }

    for (var index = start; index < end; index++) {
        let segment = decode_segment(voxel, VERTICES[index]);

        let v0 = segment.v0 * DIM_INV - 0.5;
        let v1 = segment.v1 * DIM_INV - 0.5;

        let intersection = capsule_intersection(origin, direction, v0, v1, RADIUS);

        if (intersection < HIT.distance) {
            HIT = Hit(intersection, voxel, index);
        }
    }
};

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    if (HIT.index == U32_MAX) { return vec4<f32>(0.0); }

    let segment = decode_segment(HIT.voxel, VERTICES[HIT.index]);

    let v0 = Vertex(segment.v0 * DIM_INV - 0.5, vec3<f32>(), 1.0);
    let v1 = Vertex(segment.v1 * DIM_INV - 0.5, vec3<f32>(), 1.0);

    let position = origin + direction * HIT.distance;

    return shade(v0, v1, RADIUS, position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);
}
