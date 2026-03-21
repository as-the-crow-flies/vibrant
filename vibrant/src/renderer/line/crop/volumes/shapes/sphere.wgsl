
fn in_volume(
    selection_scale: f32,
    selection_offset_x: f32,
    selection_offset_y: f32,
    selection_offset_z: f32,
    crop_length: u32,
    start: u32,
    offset_start: u32
) -> bool {
    let radius = 0.125 * selection_scale;
    let radius2 = radius * radius;

    let center = vec3<f32>(
        selection_offset_x,
        selection_offset_y,
        selection_offset_z
    );

    for (var i=0u; i<crop_length; i++) {
        let index = LINE_INDEX_RAW[start + offset_start + i];
        let v0 = LINE_VERTEX[index].xyz;
        let v1 = LINE_VERTEX[index + 1].xyz;

        let delta = v1 - v0;
        let delta2 = dot(delta, delta);

        var distance2 = 0.0;
        if (delta2 <= 0.0) {
            let distance = center - v0;
            distance2 = dot(distance, distance);
        } else {
            let u = clamp(dot(center - v0, delta) / delta2, 0.0, 1.0);
            let closest = v0 + delta * u;
            let distance = center - closest;
            distance2 = dot(distance, distance);
        }

        if (distance2 <= radius2) {
            return true;
            // total_length++;
        }
    }
    return false;
}

fn in_volume_segment(
    selection_scale: f32,
    selection_offset_x: f32,
    selection_offset_y: f32,
    selection_offset_z: f32,
    idx: u32
) -> bool {
    let radius = 0.125 * selection_scale;
    let radius2 = radius * radius;

    let center = vec3<f32>(
        selection_offset_x,
        selection_offset_y,
        selection_offset_z
    );

    let v0 = LINE_VERTEX[idx].xyz;
    let v1 = LINE_VERTEX[idx + 1].xyz;

    let delta = v1 - v0;
    let delta2 = dot(delta, delta);

    var distance2 = 0.0;
    if (delta2 <= 0.0) {
        let distance = center - v0;
        distance2 = dot(distance, distance);
    } else {
        let u = clamp(dot(center - v0, delta) / delta2, 0.0, 1.0);
        let closest = v0 + delta * u;
        let distance = center - closest;
        distance2 = dot(distance, distance);
    }

    return distance2 <= radius2;
}
