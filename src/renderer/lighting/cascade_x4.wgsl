// Cascades 2-5: four samples per thread.
use cascade;
use em/samples_x4;

fn sample_ratios(thread: Thread) -> Weights {
    let i = SAMPLES_PER_THREAD * thread.index;
    return Weights(sample_ratio(i + 0u), sample_ratio(i + 1u), sample_ratio(i + 2u), sample_ratio(i + 3u));
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
