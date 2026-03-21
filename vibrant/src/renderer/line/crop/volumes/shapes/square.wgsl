
fn in_volume(
    selection_scale: f32,
    selection_offset_x: f32,
    selection_offset_y: f32,
    selection_offset_z: f32,
    crop_length: u32,
    start: u32,
    offset_start: u32
) -> bool {
    let half_size = 0.125 * selection_scale;

    let offset = vec3<f32>(
        selection_offset_x,
        selection_offset_y,
        selection_offset_z
    );

    for (var i = 0u; i < crop_length; i++) {
        let idx = LINE_INDEX_RAW[start + offset_start + i];
        let v0 = LINE_VERTEX[idx].xyz - offset;
        let v1 = LINE_VERTEX[idx + 1].xyz - offset;

        if (
            abs(v0.x) <= half_size && abs(v0.y) <= half_size && abs(v0.z) <= half_size &&
            abs(v1.x) <= half_size && abs(v1.y) <= half_size && abs(v1.z) <= half_size
        ) {
            return true;
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
    let half_size = 0.125 * selection_scale;

    let offset = vec3<f32>(
        selection_offset_x,
        selection_offset_y,
        selection_offset_z
    );

    let v0 = LINE_VERTEX[idx].xyz - offset;
    let v1 = LINE_VERTEX[idx + 1].xyz - offset;

    return (
        abs(v0.x) <= half_size && abs(v0.y) <= half_size && abs(v0.z) <= half_size &&
        abs(v1.x) <= half_size && abs(v1.y) <= half_size && abs(v1.z) <= half_size
    );
}