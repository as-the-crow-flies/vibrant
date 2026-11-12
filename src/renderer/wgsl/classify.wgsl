// Post-classification: the visible fractions and masks, classified through
// per-fraction transfer-function lookup tables wherever they're sampled. Shared
// by the volume tracer and the cascades' extinction bake, so both agree.
use attenuation;
use color;
use uniforms;

// Per 4³ voxel block: bounds on (absorption, scattering) over every channel.
@group(1) @binding(0) var<storage, read> MAJORANT: array<vec2<f32>>;
@group(1) @binding(1) var SAMPLER: sampler;
@group(1) @binding(2) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(3) var LUT: texture_2d_array<f32>;
@group(1) @binding(4) var<uniform> CLASSIFY: Classify;
@group(1) @binding(5) var FRACTION_0: texture_3d<f32>;
@group(1) @binding(6) var FRACTION_1: texture_3d<f32>;
@group(1) @binding(7) var FRACTION_2: texture_3d<f32>;
@group(1) @binding(8) var FRACTION_3: texture_3d<f32>;
@group(1) @binding(9) var FRACTION_4: texture_3d<f32>;
@group(1) @binding(10) var FRACTION_5: texture_3d<f32>;
@group(1) @binding(11) var FRACTION_6: texture_3d<f32>;
@group(1) @binding(12) var FRACTION_7: texture_3d<f32>;
@group(1) @binding(13) var MASK_0: texture_3d<f32>;
@group(1) @binding(14) var MASK_1: texture_3d<f32>;
@group(1) @binding(15) var MASK_2: texture_3d<f32>;
@group(1) @binding(16) var MASK_3: texture_3d<f32>;
@group(1) @binding(17) var MASK_4: texture_3d<f32>;
@group(1) @binding(18) var MASK_5: texture_3d<f32>;
@group(1) @binding(19) var MASK_6: texture_3d<f32>;
@group(1) @binding(20) var MASK_7: texture_3d<f32>;
// Per COARSE³ blocks: non-zero where any of them may hold material.
@group(1) @binding(21) var<storage, read> COARSE_CELLS: array<u32>;

struct Slot {
    inverted: u32,
    masked: u32,
    mask_visible: u32,
    mask_invert: u32,
    mask_sdf: u32,
    mask_offset: f32,
    mask_width: f32,
    _pad: u32,
}

struct Classify {
    size: vec3<u32>,
    count: u32,
    crop: CropSettings,
    slots: array<Slot, 8>,
}

// The visible fractions' summed absorption, (reduced) scattering and
// density-weighted ior at `uv`, cropped. Keeps the fraction and mask values it
// read, so `sample_gradient` doesn't read them again.
fn classify(uv: vec3<f32>) -> Classified {
    var sum = Classified();

    let crop = crop_attenuation(uv);
    let n = select(0u, CLASSIFY.count, crop > 0.0);

    if (n > 0u) { sum = add_fraction(sum, 0u, FRACTION_0, MASK_0, uv); }
    if (n > 1u) { sum = add_fraction(sum, 1u, FRACTION_1, MASK_1, uv); }
    if (n > 2u) { sum = add_fraction(sum, 2u, FRACTION_2, MASK_2, uv); }
    if (n > 3u) { sum = add_fraction(sum, 3u, FRACTION_3, MASK_3, uv); }
    if (n > 4u) { sum = add_fraction(sum, 4u, FRACTION_4, MASK_4, uv); }
    if (n > 5u) { sum = add_fraction(sum, 5u, FRACTION_5, MASK_5, uv); }
    if (n > 6u) { sum = add_fraction(sum, 6u, FRACTION_6, MASK_6, uv); }
    if (n > 7u) { sum = add_fraction(sum, 7u, FRACTION_7, MASK_7, uv); }

    sum.absorption *= crop;
    sum.scattering *= crop;
    sum.crop = crop;
    return sum;
}

// Per-slot values sit in vec4 pairs (slots 0-3, 4-7): dynamically indexed
// arrays spill out of registers.
struct Classified {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    properties: vec3<f32>,
    crop: f32,
    fraction_lo: vec4<f32>,
    fraction_hi: vec4<f32>,
    mask_lo: vec4<f32>,
    mask_hi: vec4<f32>,
    // Bit k: slot k's fraction was read (skipped where its mask is zero).
    read: u32,
}

fn add_fraction(sum: Classified, k: u32, fraction_texture: texture_3d<f32>, mask_texture: texture_3d<f32>, uv: vec3<f32>) -> Classified {
    var result = sum;

    var attenuation = 1.0;
    if (has_mask(k)) {
        let mask = sample_scalar(mask_texture, uv);
        result.mask_lo = put(result.mask_lo, k, mask);
        result.mask_hi = put(result.mask_hi, k - 4u, mask);

        attenuation = attenuate(k, mask);
        if (attenuation <= 0.0) { return result; }
    }

    let fraction = sample_scalar(fraction_texture, uv);
    result.fraction_lo = put(result.fraction_lo, k, fraction);
    result.fraction_hi = put(result.fraction_hi, k - 4u, fraction);
    result.read |= 1u << k;

    let lut_uv = lut_coordinate(fraction_value(k, fraction));
    let a = textureSampleLevel(LUT, SAMPLER, lut_uv, 2u * k, 0.0);
    let s = textureSampleLevel(LUT, SAMPLER, lut_uv, 2u * k + 1u, 0.0);

    let absorption = attenuation * a.rgb;
    let scattering = attenuation * s.rgb;

    // Weight ior by density so empty materials (air) don't dilute the average.
    let density = brightness(absorption + scattering);

    result.absorption += absorption;
    result.scattering += scattering;
    result.properties += vec3<f32>(a.w, 1.0, 0.0) * density;
    return result;
}

// `v` with component `i` set to `x`; out-of-range `i` (wrapped k - 4u) leaves it.
fn put(v: vec4<f32>, i: u32, x: f32) -> vec4<f32> {
    return select(v, vec4<f32>(x), vec4<u32>(0u, 1u, 2u, 3u) == vec4<u32>(i));
}

fn pick(lo: vec4<f32>, hi: vec4<f32>, k: u32) -> f32 {
    return select(hi[k & 3u], lo[k & 3u], k < 4u);
}

// Unmasked fractions skip the mask lookup: their attenuation is 1 everywhere.
fn has_mask(k: u32) -> bool {
    let slot = CLASSIFY.slots[k];
    return bool(slot.masked) && bool(slot.mask_visible);
}

fn attenuate(k: u32, mask: f32) -> f32 {
    let slot = CLASSIFY.slots[k];
    let settings = MaskSettings(slot.mask_visible, slot.mask_invert, slot.mask_sdf, slot.mask_offset, slot.mask_width);
    return mask_attenuation(mask, settings, bool(slot.masked));
}

fn fraction_value(k: u32, fraction: f32) -> f32 {
    return select(fraction, 1.0 - fraction, bool(CLASSIFY.slots[k].inverted) && fraction != 0.0);
}

fn lut_coordinate(fraction: f32) -> vec2<f32> {
    let bins = f32(textureDimensions(LUT).x);
    return vec2<f32>((saturate(fraction) * (bins - 1.0) + 0.5) / bins, 0.5);
}

fn lut_density(k: u32, fraction: f32) -> f32 {
    return textureSampleLevel(LUT, SAMPLER, lut_coordinate(fraction), 2u * k + 1u, 0.0).w;
}

fn crop_attenuation(uv: vec3<f32>) -> f32 {
    let crop = CLASSIFY.crop;
    let inside = all(uv >= 0.5 + crop.min.xyz) && all(uv <= 0.5 + crop.max.xyz);
    return select(0.0, spherical_crop(crop, uv), inside);
}

// Gradient (per uv) of the classified density, as stored in the LUT. Each
// fraction's scalar gradient sets the direction; the LUT one voxel either side
// along it sets the magnitude, so hard transfer functions still give a normal
// within a voxel of the surface. `classified` is `classify(uv)`.
fn sample_gradient(uv: vec3<f32>, classified: Classified) -> vec3<f32> {
    let crop = crop_attenuation(uv);
    let n = select(0u, CLASSIFY.count, crop > 0.0);

    var sum = DensityGradient();
    if (n > 0u) { sum = add_gradient(sum, 0u, FRACTION_0, MASK_0, uv, classified); }
    if (n > 1u) { sum = add_gradient(sum, 1u, FRACTION_1, MASK_1, uv, classified); }
    if (n > 2u) { sum = add_gradient(sum, 2u, FRACTION_2, MASK_2, uv, classified); }
    if (n > 3u) { sum = add_gradient(sum, 3u, FRACTION_3, MASK_3, uv, classified); }
    if (n > 4u) { sum = add_gradient(sum, 4u, FRACTION_4, MASK_4, uv, classified); }
    if (n > 5u) { sum = add_gradient(sum, 5u, FRACTION_5, MASK_5, uv, classified); }
    if (n > 6u) { sum = add_gradient(sum, 6u, FRACTION_6, MASK_6, uv, classified); }
    if (n > 7u) { sum = add_gradient(sum, 7u, FRACTION_7, MASK_7, uv, classified); }

    // Crop faces: central differences of the analytic crop, a voxel apart.
    let h = 1.0 / vec3<f32>(CLASSIFY.size);
    let crop_gradient = vec3<f32>(
        crop_attenuation(uv + vec3<f32>(h.x, 0.0, 0.0)) - crop_attenuation(uv - vec3<f32>(h.x, 0.0, 0.0)),
        crop_attenuation(uv + vec3<f32>(0.0, h.y, 0.0)) - crop_attenuation(uv - vec3<f32>(0.0, h.y, 0.0)),
        crop_attenuation(uv + vec3<f32>(0.0, 0.0, h.z)) - crop_attenuation(uv - vec3<f32>(0.0, 0.0, h.z)),
    ) / (2.0 * h);

    return crop * sum.gradient + sum.density * crop_gradient;
}

struct DensityGradient {
    density: f32,
    gradient: vec3<f32>,
}

fn add_gradient(sum: DensityGradient, k: u32, fraction_texture: texture_3d<f32>, mask_texture: texture_3d<f32>, uv: vec3<f32>, classified: Classified) -> DensityGradient {
    let step = voxel_step();

    var attenuation = 1.0;
    var attenuation_gradient = vec3<f32>(0.0);

    if (has_mask(k)) {
        let mask = pick(classified.mask_lo, classified.mask_hi, k);
        let mask_gradient = sample_scalar_gradient(mask_texture, uv);
        let dm = length(mask_gradient) * step;
        attenuation = attenuate(k, mask);
        attenuation_gradient = (attenuate(k, mask + dm) - attenuate(k, mask - dm)) / (2.0 * step) * direction(mask_gradient);
    }

    var fraction = pick(classified.fraction_lo, classified.fraction_hi, k);
    if ((classified.read & (1u << k)) == 0u) { fraction = sample_scalar(fraction_texture, uv); }

    let value = fraction_value(k, fraction);
    let fraction_gradient = sample_scalar_gradient(fraction_texture, uv);
    let gradient = select(fraction_gradient, -fraction_gradient, bool(CLASSIFY.slots[k].inverted));
    let df = length(gradient) * step;

    let density = lut_density(k, value);
    let density_gradient = (lut_density(k, value + df) - lut_density(k, value - df)) / (2.0 * step) * direction(gradient);

    return DensityGradient(
        sum.density + attenuation * density,
        sum.gradient + attenuation * density_gradient + density * attenuation_gradient,
    );
}

// One voxel of the physical grid, in uv.
fn voxel_step() -> f32 {
    let size = CLASSIFY.size;
    return 1.0 / f32(max(size.x, max(size.y, size.z)));
}

fn direction(v: vec3<f32>) -> vec3<f32> {
    let l = length(v);
    return select(vec3<f32>(0.0), v / l, l > 0.0);
}

// Fractions and masks are read through a cubic B-spline: smooth surfaces
// without the trilinear creases at voxel faces.
fn sample_scalar(texture: texture_3d<f32>, uv: vec3<f32>) -> f32 {
    let texel = uv * vec3<f32>(textureDimensions(texture)) - 0.5;
    let base = floor(texel);
    let w = bspline(base, texel - base);
    return cubic_fetch(texture, w, w, w);
}

// Gradient per uv.
fn sample_scalar_gradient(texture: texture_3d<f32>, uv: vec3<f32>) -> vec3<f32> {
    let dims = vec3<f32>(textureDimensions(texture));

    let texel = uv * dims - 0.5;
    let base = floor(texel);
    let t = texel - base;

    let w = bspline(base, t);
    let d = bspline_derivative(base, t);

    return vec3<f32>(
        cubic_fetch(texture, d, w, w),
        cubic_fetch(texture, w, d, w),
        cubic_fetch(texture, w, w, d),
    ) * dims;
}

// A cubic filter folded into two linear taps per axis (Sigg & Hadwiger 2005):
// weights of texels i-1 + i and i+1 + i+2, and the positions between them that
// blend each pair in the right ratio.
struct Taps {
    w0: vec3<f32>,
    w1: vec3<f32>,
    p0: vec3<f32>,
    p1: vec3<f32>,
}

fn bspline(base: vec3<f32>, t: vec3<f32>) -> Taps {
    let s = 1.0 - t;
    let w0 = s * s * s / 6.0;
    let w1 = (3.0 * t * t * t - 6.0 * t * t + 4.0) / 6.0;
    let w2 = (-3.0 * t * t * t + 3.0 * t * t + 3.0 * t + 1.0) / 6.0;
    let w3 = t * t * t / 6.0;
    return taps(base, w0, w1, w2, w3);
}

// Both pairs keep a single sign, so the same folding works.
fn bspline_derivative(base: vec3<f32>, t: vec3<f32>) -> Taps {
    let s = 1.0 - t;
    let w0 = -0.5 * s * s;
    let w1 = 0.5 * t * (3.0 * t - 4.0);
    let w2 = 0.5 * s * (3.0 * t + 1.0);
    let w3 = 0.5 * t * t;
    return taps(base, w0, w1, w2, w3);
}

fn taps(base: vec3<f32>, w0: vec3<f32>, w1: vec3<f32>, w2: vec3<f32>, w3: vec3<f32>) -> Taps {
    let g0 = w0 + w1;
    let g1 = w2 + w3;
    return Taps(g0, g1, base - 1.0 + w1 / g0, base + 1.0 + w3 / g1);
}

// x, y and z each contribute their own axis of their taps.
fn cubic_fetch(texture: texture_3d<f32>, x: Taps, y: Taps, z: Taps) -> f32 {
    let inverse = 1.0 / vec3<f32>(textureDimensions(texture));
    let lo = (vec3<f32>(x.p0.x, y.p0.y, z.p0.z) + 0.5) * inverse;
    let hi = (vec3<f32>(x.p1.x, y.p1.y, z.p1.z) + 0.5) * inverse;

    let z0 =
        y.w0.y * (x.w0.x * tap(texture, lo.x, lo.y, lo.z) + x.w1.x * tap(texture, hi.x, lo.y, lo.z)) +
        y.w1.y * (x.w0.x * tap(texture, lo.x, hi.y, lo.z) + x.w1.x * tap(texture, hi.x, hi.y, lo.z));
    let z1 =
        y.w0.y * (x.w0.x * tap(texture, lo.x, lo.y, hi.z) + x.w1.x * tap(texture, hi.x, lo.y, hi.z)) +
        y.w1.y * (x.w0.x * tap(texture, lo.x, hi.y, hi.z) + x.w1.x * tap(texture, hi.x, hi.y, hi.z));

    return z.w0.z * z0 + z.w1.z * z1;
}

fn tap(texture: texture_3d<f32>, x: f32, y: f32, z: f32) -> f32 {
    return textureSampleLevel(texture, SAMPLER, vec3<f32>(x, y, z), 0.0).x;
}

const BLOCK: u32 = 4u;
// Blocks per coarse cell, per axis.
const COARSE: u32 = 8u;

fn block_count() -> vec3<u32> {
    return (CLASSIFY.size + BLOCK - 1u) / BLOCK;
}

// Bounds the classified extinction of every channel, with scattering scaled
// by `scattering_scale`.
fn block_majorant(block: vec3<u32>, scattering_scale: f32) -> f32 {
    let blocks = block_count();
    let bound = MAJORANT[block.x + blocks.x * (block.y + blocks.y * block.z)];
    return bound.x + scattering_scale * bound.y;
}

fn coarse_count() -> vec3<u32> {
    return (block_count() + COARSE - 1u) / COARSE;
}

fn coarse_empty(cell: vec3<u32>) -> bool {
    let cells = coarse_count();
    return COARSE_CELLS[cell.x + cells.x * (cell.y + cells.y * cell.z)] == 0u;
}
