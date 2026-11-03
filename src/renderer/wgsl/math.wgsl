const TRUE: u32 = 1u;
const FALSE: u32 = 0u;

const U32_MAX: u32 = 4294967295;

const U16_SHIFT: u32 = 16;
const U16_MAX: u32 = 65535;
const U16_MAX_f32: f32 = f32(U16_MAX);
const U16_MAX_INV: f32 = 1.0 / U16_MAX_f32;

const PI: f32 = 3.14159265358979323846264338327950288;

fn sign_not_zero(v: f32) -> f32 {
  return select(-1.0, 1.0, v >= 0.0);
}

fn sign_not_zero_2(v: vec2<f32>) -> vec2<f32> {
  return vec2<f32>(select(-1.0, 1.0, v.x >= 0.0), select(-1.0, 1.0, v.y >= 0.0));
}

fn rotation_y(angle: f32) -> mat3x3<f32> {
    let c = cos(angle);
    let s = sin(angle);

    return mat3x3<f32>(
        vec3<f32>( c, 0.0, -s),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>( s, 0.0,  c)
    );
}

fn rotation_z(angle: f32) -> mat3x3<f32> {
    let c = cos(angle);
    let s = sin(angle);

    return mat3x3<f32>(
        vec3<f32>( c,  s, 0.0),
        vec3<f32>(-s,  c, 0.0),
        vec3<f32>(0.0, 0.0, 1.0)
    );
}

fn orthonormalize(normal: vec3<f32>, tangent: vec3<f32>) -> vec3<f32> {
    return normalize(normal - dot(normal, tangent) * tangent);
}
