@group(0) @binding(0) var<storage, read_write> SEED: array<vec4<f32>>;

// K lobes at uniform Hammersley directions for every power-of-two K, at SEED[K..2K].
@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i == 0u || i >= arrayLength(&SEED)) { return; }

    let count = 1u << firstLeadingBit(i);
    let direction = octahedron_decode(2.0 * hammersley(i - count, count) - 1.0);

    SEED[i] = vec4<f32>(direction, 1.0) / f32(count);
}
