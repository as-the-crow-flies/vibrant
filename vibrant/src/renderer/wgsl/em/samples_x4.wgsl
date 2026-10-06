const SAMPLES_PER_THREAD: u32 = 4u;

alias Directions = mat4x3<f32>;
alias Radiances = mat4x3<f32>;
alias Weights = vec4<f32>;

fn hammersley_directions(thread: Thread) -> Directions {
    let i = SAMPLES_PER_THREAD * thread.index;
    return Directions(
        hammersley_direction(i + 0u, SAMPLES),
        hammersley_direction(i + 1u, SAMPLES),
        hammersley_direction(i + 2u, SAMPLES),
        hammersley_direction(i + 3u, SAMPLES),
    );
}

fn trace_samples(origin: vec3<f32>, omega: Directions) -> Radiances {
    return Radiances(
        trace(origin, omega[0]),
        trace(origin, omega[1]),
        trace(origin, omega[2]),
        trace(origin, omega[3]),
    );
}

fn radiance_weights(radiance: Radiances) -> Weights {
    return Weights(
        radiance_weight(radiance[0]),
        radiance_weight(radiance[1]),
        radiance_weight(radiance[2]),
        radiance_weight(radiance[3]),
    );
}

fn project(v: vec3<f32>, omega: Directions) -> Weights {
    return v * omega;
}

fn total(w: Weights) -> f32 {
    return w.x + w.y + w.z + w.w;
}

fn outer(v: vec3<f32>, w: Weights) -> Radiances {
    return Radiances(v * w[0], v * w[1], v * w[2], v * w[3]);
}

fn scale(radiance: Radiances, w: Weights) -> Radiances {
    return Radiances(radiance[0] * w[0], radiance[1] * w[1], radiance[2] * w[2], radiance[3] * w[3]);
}

fn hadamard(a: Radiances, b: Radiances) -> Radiances {
    return Radiances(a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]);
}
