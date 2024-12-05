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

    let v0 = TRACTOGRAM_TO_WORLD * get_vertex(index);
    let v1 = TRACTOGRAM_TO_WORLD * get_vertex(index + 1);

    let tangent = abs(normalize(v1.xyz - v0.xyz));

    let start = transform(v0);
    let end = transform(v1);

    let bounds_min = vec2<f32>(0.0);
    let bounds_max = vec2<f32>(f32(SURFACE_X), f32(SURFACE_Y));

    if (start.z > 0.0 && start.z < 1.0 && end.z > 0.0 && end.z < 1.0 &&
        (
            (all(start.xy >= bounds_min) && all(start.xy < bounds_max)) ||
            (all(  end.xy >= bounds_min) && all(  end.xy < bounds_max)))
        )
    {
        let delta = end - start;
        let steps = maximum(abs(delta.xy));
        let step = delta / steps;
                                                                            //  0   1   2   3
        let work_generated = u32(steps) + 1u;                               //  8   2   7   5
        let work_offset = subgroupExclusiveAdd(work_generated);             //  0   8   10  17
        let work_total = subgroupAdd(work_generated);                       //  22

        let thread_total = work_total / subgroup_size +                     //  6   6   5   5
            u32(subgroup_id < (work_total % subgroup_size));

        var thread_offset = subgroupExclusiveAdd(thread_total);             //  0   6   12  17
        var thread_max = thread_offset + thread_total;                      //  6   12  17  22
        var work_index = binary_search(work_offset, thread_offset);         //  0   0   2   3

        // Loop Variables
        var work_offset_ = subgroupShuffle(work_offset, work_index);
        var start_ = subgroupShuffle(start, work_index);
        var step_ = subgroupShuffle(step, work_index);
        var payload_ = u64(pack4x8unorm(vec4<f32>(subgroupShuffle(tangent, work_index), 1.0)));

        for (; thread_offset<thread_max; thread_offset++) {
            let offset = thread_offset - work_offset_;
            let sample = start_ + step_ * f32(offset);

            let depth = u64(sample.z * f32(U32_MAX));
            let visibility = depth << 32u | payload_;

            atomicMin(&KBUFFER[u32(sample.y)][u32(sample.x)], visibility);

            // Increment Primitive if necessary
            if (offset == subgroupShuffle(work_generated, work_index) - 1) {
                work_index++;

                work_offset_ = subgroupShuffle(work_offset, work_index);
                start_ = subgroupShuffle(start, work_index);
                step_ = subgroupShuffle(step, work_index);
                payload_ = u64(pack4x8unorm(vec4<f32>(subgroupShuffle(tangent, work_index), 1.0)));
            }
        }
    }
}

fn binary_search(a: u32, k: u32) -> u32 {
    var low = 0u;
    var high = 31u;
    var ans = 32u;

    while (low <= high) {
        let mid = low + (high - low) / 2;

        if (subgroupShuffle(a, mid) <= k) {
            low = mid+1u;
        }
        else {
            ans = mid;
            high = mid - 1u;
        }
    }

    return ans - 1u;
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
