// Cascades 0-1: one sample per thread.
use cascade;
use em/samples_x1;

fn sample_ratios(thread: Thread) -> Weights {
    return sample_ratio(thread.index);
}

fn trace_samples(origin: vec3<f32>, omega: Directions, ratio: Weights) -> Radiances {
    return trace(origin, omega, ratio);
}

fn radiance_weights(radiance: Radiances) -> Weights {
    return radiance_weight(radiance);
}
