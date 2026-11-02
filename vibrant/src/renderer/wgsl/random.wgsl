fn hash(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn van_der_corput(i: u32) -> f32 {
    return f32(reverseBits(i)) * 2.3283064365386963e-10; // 1 / 2^32
}

fn hammersley(i: u32, N: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(N), van_der_corput(i));
}
