
fn in_rectangle_volume(
    selection_offset_x: f32,
    selection_offset_y: f32,
    selection_offset_z: f32,
    size_x: f32,
    size_y: f32,
    size_z: f32,
    crop_length: u32,
    start: u32,
    offset_start: u32
) -> bool {
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
            (abs(v0.x) <= size_x && abs(v0.y) <= size_y && abs(v0.z) <= size_z) ||
            (abs(v1.x) <= size_x && abs(v1.y) <= size_y && abs(v1.z) <= size_z)
        ) {
            return true;
        }
    }

    return false;
}

fn in_rectangle_volume_segment(
    selection_offset_x: f32,
    selection_offset_y: f32,
    selection_offset_z: f32,
    size_x: f32,
    size_y: f32,
    size_z: f32,
    idx: u32
) -> bool {
    let offset = vec3<f32>(
        selection_offset_x,
        selection_offset_y,
        selection_offset_z
    );

    let v0 = LINE_VERTEX[idx].xyz - offset;
    let v1 = LINE_VERTEX[idx + 1].xyz - offset;

    return (
        abs(v0.x) <= size_x && abs(v0.y) <= size_y && abs(v0.z) <= size_z &&
        abs(v1.x) <= size_x && abs(v1.y) <= size_y && abs(v1.z) <= size_z
    );
}

//DISPATCH-INSERT-MARKER//
