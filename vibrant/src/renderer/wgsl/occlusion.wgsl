// Occluders of a radiance buffer: combined = (1, 1); overlay volume cascade =
// (1, 0); overlay line cascade = (0, 1).
struct CascadeOpts {
    include_volume: u32,
    include_lines: u32,
}

// Line occlusion falls off as w(t) = min(1, (R/t)²) with distance t (mm), so
// nearby structure dominates and total line optical depth is bounded by 2Rσ,
// independent of the probe resolution.
const LINE_FALLOFF_MM: f32 = 4.0;

// ∫ w over [0, t].
fn line_falloff_depth(t: f32) -> f32 {
    let r = LINE_FALLOFF_MM;
    return select(2.0 * r - r * r / t, t, t <= r);
}
