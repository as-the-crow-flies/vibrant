// Occluders of a radiance buffer: combined = (1, 1); overlay volume cascade =
// (1, 0); overlay line cascade = (0, 1).
struct CascadeOpts {
    include_volume: u32,
    include_lines: u32,
}
