fn brightness(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    return select(
        1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055,
        linear * 12.92,
        linear < vec3<f32>(0.0031308)
    );
}

fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    return select(
        pow((srgb + 0.055) / 1.055, vec3<f32>(2.4)),
        srgb / 12.92,
        srgb < vec3<f32>(0.04045)
    );
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return saturate((x * (a * x + b)) / (x * (c * x + d) + e));
}

/*
contributors: Bjorn Ottosson (@bjornornorn)
description: Oklab to linear RGB https://bottosson.github.io/posts/oklab/
license:
    - MIT License (MIT) Copyright (c) 2020 Björn Ottosson
*/

const OKLAB2RGB_A : mat3x3<f32>  = mat3x3<f32>(
    vec3f(1.0, 1.0, 1.0),
    vec3f(0.3963377774, -0.1055613458, -0.0894841775),
    vec3f(0.2158037573, -0.0638541728, -1.2914855480) );

const OKLAB2RGB_B : mat3x3<f32>  = mat3x3<f32>(
    vec3f(4.0767416621, -1.2684380046, -0.0041960863),
    vec3f(-3.3077115913, 2.6097574011, -0.7034186147),
    vec3f(0.2309699292, -0.3413193965, 1.7076147010) );

fn oklab2rgb(oklab: vec3f) -> vec3f {
    let lms = OKLAB2RGB_A * oklab;
    return OKLAB2RGB_B * (lms * lms * lms);
}

fn tangent2rgb(tangent: vec3<f32>) -> vec3<f32> {
    // Hue directions in the OkLab (a, b) plane for the three tract axes.
    let red = vec2<f32>(0.217, 0.125);
    let green = vec2<f32>(-0.217, 0.125);
    let blue = vec2<f32>(0.000, -0.250);

    let lightness = 0.8;
    let chroma = 0.8;
    let ab = tangent.r * red + tangent.g * green + tangent.b * blue;

    return oklab2rgb(vec3<f32>(lightness, chroma * ab));
}

const PACK_RGB_LOG_LO:   f32 = -9.965784;  // log2(1e-3)
const PACK_RGB_LOG_SPAN: f32 = 16.609640;  // log2(1e2) - log2(1e-3)

fn pack_rgb(src: vec3<f32>) -> vec4<f32> {
    let maxc = max(src.r, max(src.g, src.b));
    if (maxc <= 0.0) { return vec4<f32>(0.0); }
    let e = clamp(log2(maxc), PACK_RGB_LOG_LO, PACK_RGB_LOG_LO + PACK_RGB_LOG_SPAN);
    // quantize the scale UP to the 8-bit grid so scale >= maxc (no clipping)
    let a = clamp(ceil((e - PACK_RGB_LOG_LO) / PACK_RGB_LOG_SPAN * 255.0) / 255.0, 0.0, 1.0);
    let scale = exp2(PACK_RGB_LOG_LO + a * PACK_RGB_LOG_SPAN);
    return vec4<f32>(src / scale, a);   // mantissa in (0.956, 1] for the max channel
}

fn unpack_rgb(src: vec4<f32>) -> vec3<f32> {
    let scale = exp2(PACK_RGB_LOG_LO + src.a * PACK_RGB_LOG_SPAN);
    return src.rgb * scale;
}
