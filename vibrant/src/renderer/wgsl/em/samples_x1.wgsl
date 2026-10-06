const SAMPLES_PER_THREAD: u32 = 1u;

alias Directions = vec3<f32>;
alias Radiances = vec3<f32>;
alias Weights = f32;

fn hammersley_directions(thread: Thread) -> Directions {
    return hammersley_direction(thread.index, SAMPLES);
}

fn trace_samples(origin: vec3<f32>, omega: Directions) -> Radiances {
    return trace(origin, omega);
}

fn radiance_weights(radiance: Radiances) -> Weights {
    return radiance_weight(radiance);
}

fn project(v: vec3<f32>, omega: Directions) -> Weights {
    return dot(v, omega);
}

fn total(w: Weights) -> f32 {
    return w;
}

fn outer(v: vec3<f32>, w: Weights) -> Radiances {
    return v * w;
}

fn scale(radiance: Radiances, w: Weights) -> Radiances {
    return radiance * w;
}

fn hadamard(a: Radiances, b: Radiances) -> Radiances {
    return a * b;
}
