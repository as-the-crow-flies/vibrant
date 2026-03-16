use vibrant::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum TestSelectionVolume {
    None,
    Box,
    Sphere,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TestSegment {
    start: Vec3,
    end: Vec3,
}

fn test_segment(start: [f32; 3], end: [f32; 3]) -> TestSegment {
    TestSegment {
        start: Vec3::from_array(start),
        end: Vec3::from_array(end),
    }
}

fn half_size_from_scale(selection_scale: f32) -> f32 {
    0.125 * selection_scale
}

fn sphere_radius_from_scale(selection_scale: f32) -> f32 {
    0.125 * selection_scale
}

fn endpoint_inside_box(point: Vec3, center: Vec3, half_size: f32) -> bool {
    let local = point - center;
    local.x.abs() <= half_size && local.y.abs() <= half_size && local.z.abs() <= half_size
}

fn segment_point_distance2(v0: Vec3, v1: Vec3, point: Vec3) -> f32 {
    let delta = v1 - v0;
    let delta2 = delta.length_squared();

    if delta2 <= f32::EPSILON {
        return (point - v0).length_squared();
    }

    let t = ((point - v0).dot(delta) / delta2).clamp(0.0, 1.0);
    let closest = v0 + delta * t;
    (point - closest).length_squared()
}

fn apply_box_selection(
    cropped_segments: &[TestSegment],
    selection_scale: f32,
    selection_offset: Vec3,
    selection_extend_lines: bool,
) -> Vec<TestSegment> {
    if selection_scale <= 0.0 {
        return cropped_segments.to_vec();
    }

    let half_size = half_size_from_scale(selection_scale);

    let selected: Vec<_> = cropped_segments
        .iter()
        .copied()
        .filter(|segment| {
            endpoint_inside_box(segment.start, selection_offset, half_size)
                && endpoint_inside_box(segment.end, selection_offset, half_size)
        })
        .collect();

    if selection_extend_lines {
        if selected.is_empty() {
            Vec::new()
        } else {
            cropped_segments.to_vec()
        }
    } else {
        selected
    }
}

fn apply_sphere_selection(
    cropped_segments: &[TestSegment],
    selection_scale: f32,
    selection_offset: Vec3,
    selection_extend_lines: bool,
) -> Vec<TestSegment> {
    if selection_scale <= 0.0 {
        return cropped_segments.to_vec();
    }

    let radius = sphere_radius_from_scale(selection_scale);
    let radius2 = radius * radius;

    let selected: Vec<_> = cropped_segments
        .iter()
        .copied()
        .filter(|segment| {
            segment_point_distance2(segment.start, segment.end, selection_offset) <= radius2
        })
        .collect();

    if selection_extend_lines {
        if selected.is_empty() {
            Vec::new()
        } else {
            cropped_segments.to_vec()
        }
    } else {
        selected
    }
}

fn apply_selection_volume(
    mode: TestSelectionVolume,
    cropped_segments: &[TestSegment],
    selection_scale: f32,
    selection_offset: Vec3,
    selection_extend_lines: bool,
) -> Vec<TestSegment> {
    match mode {
        TestSelectionVolume::None => cropped_segments.to_vec(),
        TestSelectionVolume::Box => apply_box_selection(
            cropped_segments,
            selection_scale,
            selection_offset,
            selection_extend_lines,
        ),
        TestSelectionVolume::Sphere => apply_sphere_selection(
            cropped_segments,
            selection_scale,
            selection_offset,
            selection_extend_lines,
        ),
    }
}

#[test]
fn selection_volume_none_is_identity() {
    let cropped_segments = vec![
        test_segment([-0.4, 0.0, 0.0], [-0.2, 0.0, 0.0]),
        test_segment([0.0, 0.0, 0.0], [0.2, 0.0, 0.0]),
        test_segment([0.3, 0.2, 0.0], [0.4, 0.3, 0.0]),
    ];

    let selected = apply_selection_volume(
        TestSelectionVolume::None,
        &cropped_segments,
        1.0,
        Vec3::new(0.1, -0.1, 0.0),
        true,
    );

    assert_eq!(selected, cropped_segments);
}

#[test]
fn selection_volume_box_mode_preserves_existing_box_behavior() {
    let inside = test_segment([-0.05, 0.0, 0.0], [0.05, 0.0, 0.0]);
    let intersects_but_endpoint_outside = test_segment([0.0, 0.0, 0.0], [0.2, 0.0, 0.0]);
    let outside = test_segment([0.3, 0.3, 0.0], [0.4, 0.3, 0.0]);
    let cropped_segments = vec![inside, intersects_but_endpoint_outside, outside];

    let selected_without_extend = apply_selection_volume(
        TestSelectionVolume::Box,
        &cropped_segments,
        1.0,
        Vec3::ZERO,
        false,
    );
    assert_eq!(selected_without_extend, vec![inside]);

    let selected_with_extend = apply_selection_volume(
        TestSelectionVolume::Box,
        &cropped_segments,
        1.0,
        Vec3::ZERO,
        true,
    );
    assert_eq!(selected_with_extend, cropped_segments);
}

#[test]
fn selection_volume_sphere_scale_maps_to_box_half_size() {
    let selection_scale = 2.0;
    let expected_radius = half_size_from_scale(selection_scale);
    let actual_radius = sphere_radius_from_scale(selection_scale);
    assert_eq!(actual_radius, expected_radius);

    let within_radius = test_segment([0.24, 0.0, 0.0], [0.24, 0.01, 0.0]);
    let outside_radius = test_segment([0.26, 0.0, 0.0], [0.26, 0.01, 0.0]);

    let selected = apply_selection_volume(
        TestSelectionVolume::Sphere,
        &[within_radius, outside_radius],
        selection_scale,
        Vec3::ZERO,
        false,
    );

    assert_eq!(selected, vec![within_radius]);
}

#[test]
fn selection_volume_sphere_extend_lines_keeps_full_line_after_hit() {
    let hit = test_segment([-0.2, 0.0, 0.0], [0.2, 0.0, 0.0]);
    let miss_a = test_segment([0.4, 0.3, 0.0], [0.5, 0.3, 0.0]);
    let miss_b = test_segment([-0.5, -0.3, 0.0], [-0.4, -0.3, 0.0]);
    let cropped_segments = vec![hit, miss_a, miss_b];

    let selected = apply_selection_volume(
        TestSelectionVolume::Sphere,
        &cropped_segments,
        1.0,
        Vec3::ZERO,
        true,
    );

    assert_eq!(selected, cropped_segments);
}

#[test]
fn selection_volume_sphere_without_extend_keeps_only_intersecting_segments() {
    let crossing_hit = test_segment([-0.2, 0.0, 0.0], [0.2, 0.0, 0.0]);
    let endpoint_hit = test_segment([0.12, 0.0, 0.0], [0.2, 0.0, 0.0]);
    let miss = test_segment([0.3, 0.3, 0.0], [0.4, 0.3, 0.0]);

    let selected = apply_selection_volume(
        TestSelectionVolume::Sphere,
        &[crossing_hit, endpoint_hit, miss],
        1.0,
        Vec3::ZERO,
        false,
    );

    assert_eq!(selected, vec![crossing_hit, endpoint_hit]);
}
