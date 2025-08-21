const LOCAL_SORT_SIZE: u32 = 32;

var<private> COLOR: vec4<f32>;

fn visit(
    count: u32,
    offset: u32,
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction: vec3<f32>,
    position: vec3<f32>,
    increment: f32,
    distance: f32) -> bool {

    let increment_inv = 1.0 / increment;

    var hit_count = 0u;
    var hits = array<u32, LOCAL_SORT_SIZE>();

    for (var i = 0u; i < count; i++) {
        let index = INDEX[offset + i];

        let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
        let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

        let hit = capsule_intersection(position, direction, v0.xyz, v1.xyz, RADIUS);
        let hit_position = position + hit * direction;

        if (hit < 0.0 || hit >= increment || should_be_clipped(v0, v1, hit_position)) { continue; }

        let candidate = (u32((hit * increment_inv) * U16_MAX_f32) << 16) | i;

        insert_hit(&hits, hit_count, candidate);

        hit_count++;
    }

    if (hit_count > 0) {
        for (var i=0u; i<hit_count; i++) {
            let item = hits[i];
            let index = INDEX[offset + (item & U16_MAX)];

            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            let hit = f32(item >> 16) * U16_MAX_INV * increment;
            let hit_position = position + hit * direction;

            let c = shade(v0, v1, RADIUS, hit_position, direction, hit_position * DIM_INV, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

            COLOR += (1.0 - COLOR.a) * vec4<f32>(c.rgb * c.a, c.a);

            if (COLOR.a > 0.95) { break; }
        }
    }

    return COLOR.a > 0.95;
}

fn result(origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    return COLOR;
}

fn sort(data: ptr<function, array<u32, LOCAL_SORT_SIZE>>, count: u32) {
    for (var i: u32 = 1u; i < count; i = i + 1u) {
        let key = (*data)[i];
        var j: i32 = i32(i) - 1;

        while (j >= 0 && (*data)[u32(j)] > key) {
            (*data)[u32(j + 1)] = (*data)[u32(j)];
            j = j - 1;
        }

        (*data)[u32(j + 1)] = key;
    }
}

fn binary_search_insert_index(hits: ptr<function, array<u32, LOCAL_SORT_SIZE>>, hit_count: u32, value: u32) -> u32 {
    var lo: u32 = 0u;
    var hi: u32 = hit_count;

    while (lo < hi) {
        let mid: u32 = (lo + hi) / 2u;
        if ((*hits)[mid] < value) {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }

    return lo;
}

fn insert_hit(hits: ptr<function, array<u32, LOCAL_SORT_SIZE>>, hit_count: u32, value: u32) {
    let hit_count_clamped = min(hit_count, LOCAL_SORT_SIZE);

    let idx = binary_search_insert_index(hits, hit_count_clamped, value);

    // Shift elements to make room
    for (var i = hit_count_clamped; i > idx; i--) {
        (*hits)[i] = (*hits)[i - 1u];
    }

    // Insert and increment count
    (*hits)[idx] = value;
}
