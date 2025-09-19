@group(2) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(2) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;

@group(3) @binding(0) var<storage> OFFSET: array<u32>;
@group(3) @binding(2) var<storage> INDEX: array<u32>;

var<private> COLOR: vec4<f32>;

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

    let increment_inv = 1.0 / increment;

    var min_depth = 0.0;

    loop {
        var hits = array<u32, INSERTION_SORT_SIZE>();
        var hit_count = 0u;

        for (var i = 0u; i < count; i++) {
            let index = INDEX[offset + i];

            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            let hit = capsule_intersection(position, direction, v0.xyz, v1.xyz, RADIUS);
            let hit_position = position + hit * direction;

            if (hit < min_depth || hit >= increment || should_be_clipped(v0, v1, hit_position)) { continue; }

            let candidate = (u32((hit * increment_inv) * U16_MAX_f32) << 16) | i;

            insertion_sort_insert(&hits, hit_count, candidate);

            hit_count++;
        }

        let hit_count_clamped = min(hit_count, INSERTION_SORT_SIZE);

        for (var i=0u; i<hit_count_clamped; i++) {
            let item = hits[i];
            let index = INDEX[offset + (item & U16_MAX)];

            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            let hit = f32(item >> 16u) * U16_MAX_INV * increment;
            let hit_position = position + hit * direction;

            let c = shade(v0, v1, RADIUS, hit_position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

            COLOR += (1.0 - COLOR.a) * vec4<f32>(c.rgb * c.a, c.a);

            if (COLOR.a > 0.95) { break; }
        }

        if (hit_count == hit_count_clamped || COLOR.a > 0.95) { break; }

        min_depth = f32(hits[INSERTION_SORT_SIZE - 1u] >> 16u) * U16_MAX_INV * increment;
    }

    return COLOR.a > 0.95;
}

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    return COLOR;
}
