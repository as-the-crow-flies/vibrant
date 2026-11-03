// Occluders of a radiance buffer: combined = (1, 1); overlay volume cascade =
// (1, 0); overlay line cascade = (0, 1). Plus the lobe count and, for the
// cascade pass, the level being solved.
struct CascadeOpts {
    include_volume: u32,
    include_lines: u32,
    vmm_size: u32,
    cascade: u32,
}
