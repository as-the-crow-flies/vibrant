struct Camera {
    transform: mat4x4<f32>,
    projection_inverse: mat4x4<f32>,
}

struct Environment {
    camera: Camera,
    time: f32,
}

// Tractography-only render parameters (see asset/tractography.rs). Bound by the
// line passes; field order must match `TractographySettings`.
struct Tractography {
    volume: u32,
    // 0 = Combined, 1 = Overlay.
    render_mode: u32,
    // Whether the slicing / clipping planes cull tractography lines.
    line_crop: u32,
    radius: f32,
    alpha: f32,
    ambient_light: f32,
    direct_light: f32,
    smoothing: f32,
    crop_start: f32,
    crop_end: f32,
    line_roughness: f32,
    line_specular: f32,
    line_translucency: f32,
    line_occlusion: f32,
    line_extinction: f32,
}

struct LineSettings {
    visible: u32,
    color: u32,
    colormap: u32,
    crop_start: f32,
    crop_end: f32,
    // Whether the slicing / clipping planes cut this line. Only takes
    // effect while `Tractography.line_crop` (the master switch) is also on.
    crop: u32,
}

// One cell of the environment's cut, see asset/hdri/importance/grid.rs.
struct EnvironmentCell {
    direction: vec3<f32>,
    // Solid angle over that of a uniform sample.
    ratio: f32,
    color: vec3<f32>,
    // Average EM weight.
    weight: f32,
}

struct HdriSettings {
    rotation: f32,
    strength: f32,
    specular: f32,
    // Strength of the volume's scattered light.
    phase: f32,
    roughness: f32,
    anisotropy: f32,
    importance: u32,
    // Cascade samples read the parent's lobes blurred by their footprint.
    prefilter: u32,
    log_weights: u32,
}

struct CropSettings {
    spherical: vec4<f32>,
    min: vec4<f32>,
    max: vec4<f32>,
}
