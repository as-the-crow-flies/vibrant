// Used instead of vec3<f32> for padding reasons
struct Vertex {
    x: f32,
    y: f32,
    z: f32
}

@group(0) @binding(0) var<storage, read_write> KBUFFER: array<array<atomic<u64>, SURFACE_X>, SURFACE_Y>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<Vertex>;

@group(2) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;
@group(2) @binding(1) var<storage> TRACTOGRAM_INDICES_COUNT: u32;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const U32_MAX: u32 = 4294967295;

fn get_vertex(index: u32) -> vec4<f32> {
    let v = TRACTOGRAM_VERTICES[index];
    return vec4<f32>(v.x, v.y, v.z, 1.0);
}

@compute
@workgroup_size(WORKGROUP_X)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(subgroup_invocation_id) subgroup_id: u32,
    @builtin(subgroup_size) subgroup_size: u32) {
    if (global_id.x >= TRACTOGRAM_INDICES_COUNT) { return; }

    let index = TRACTOGRAM_INDICES[global_id.x];

    let v0_world = TRACTOGRAM_TO_WORLD * get_vertex(index);
    let v1_world = TRACTOGRAM_TO_WORLD * get_vertex(index + 1);

    let tangent = abs(normalize(v1_world.xyz - v0_world.xyz));

    let start = transform(v0_world);
    let end = transform(v1_world);

    if (start.z <= 0.0 || end.z <= 0.0) { return; } // Safety Feature :)

    let delta = end - start;
    let steps = maximum(abs(delta.xy));
    let step = delta / steps;

    if (bool(ENVIRONMENT.settings.balancing)) {                             //  0   1   2   3  <- Thread Id
        let size = u32(steps);                                              //  8   2   7   5
        let offset = subgroupExclusiveAdd(size);                            //  0   8   10  17
        let total = subgroupBroadcast(offset + size, subgroup_size - 1);    //  22

        let thread_min = total / subgroup_size;
        let thread_mod = total % subgroup_size;

        let thread_total = thread_min + u32(subgroup_id < thread_mod);                  //  6   6   5   5
        var thread_offset = subgroup_id * thread_min + min(subgroup_id, thread_mod);    //  0   6   12  17
        var thread_index = search(subgroup_size, offset, thread_offset);                //  0   0   2   3
        var thread_offset_max = thread_offset + thread_total;                           //  6   12  17  22

        for (; thread_offset<thread_offset_max; thread_offset++) {
            let offset_ = subgroupShuffle(offset, thread_index);
            let start_ = subgroupShuffle(start, thread_index);
            let step_ = subgroupShuffle(step, thread_index);
            let tangent_ = subgroupShuffle(tangent, thread_index);
            let size_ = subgroupShuffle(size, thread_index);

            let primitive_offset = thread_offset - offset_;
            let fragment = start_ + step_ * f32(primitive_offset);

            let depth = u64(fragment.z * f32(U32_MAX)) << 32u;
            let payload = u64(pack4x8unorm(vec4<f32>(tangent_, 0.0)));

            let pixel = &KBUFFER[u32(fragment.y)][u32(fragment.x)];

            atomicMin(pixel, depth | payload);

            if (primitive_offset >= size_ - 1) { thread_index++; }
        }
    } else {
        for (var offset = 0.0; offset <= steps; offset += 1.0) {
            let fragment = start + step * offset;

            let depth = u64(fragment.z * f32(U32_MAX)) << 32u;
            let payload = u64(pack4x8unorm(vec4<f32>(tangent, 0.0)));

            let pixel = &KBUFFER[u32(fragment.y)][u32(fragment.x)];

            atomicMin(pixel, depth | payload);
        }
    }
}

fn search(size: u32, offsets: u32, k: u32) -> u32 {
    var low = 0u;
    var high = size - 1u;

    while (low <= high) {
        let mid = low + (high - low) / 2;
        let offset = subgroupShuffle(offsets, mid);

        if (offset <= k) { low = mid + 1u; }
        else { high = mid - 1u; }
    }

    return high;
}

fn transform(vertex: vec4<f32>) -> vec3<f32> {
    let v = ENVIRONMENT.camera.projection * vertex;
    let vt = v.xyz / v.w * 0.5 + 0.5;
    return vec3<f32>(vt.x * f32(SURFACE_X), (1.0 - vt.y) * f32(SURFACE_Y), vt.z);
}

fn maximum(v: vec2<f32>) -> f32 {
    return max(v.x, v.y);
}

fn minimum(v: vec2<f32>) -> f32 {
    return min(v.x, v.y);
}
