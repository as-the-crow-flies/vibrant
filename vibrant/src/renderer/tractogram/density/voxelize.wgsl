@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> WORLD_TO_VOLUME: mat4x4<f32>;

@group(0) @binding(2) var<storage, read_write> BIN_COUNTER: atomic<u32>;
@group(0) @binding(3) var<storage, read_write> BIN_HEADS: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> BIN_PREVIOUS: array<u32>;
@group(0) @binding(5) var<storage, read_write> BIN_INDICES: array<u32>;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var<storage> TRACTOGRAM_INDICES: array<u32>;

const PI: f32 = 3.14159265358979323846264338327950288;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(workgroup_id) workgroup: vec3<u32>,
    @builtin(local_invocation_index) local: u32
) {
    let transform = WORLD_TO_VOLUME * TRACTOGRAM_TO_WORLD;
    let radius = ENVIRONMENT.settings.streamline_radius;

    let n_indices = arrayLength(&TRACTOGRAM_INDICES);

    let offset = workgroup.x * WORKGROUP_SIZE * CHUNK_SIZE;

    for (var chunk=0u; chunk<CHUNK_SIZE; chunk++) {
        let index = TRACTOGRAM_INDICES[min(offset + chunk * WORKGROUP_SIZE + local, n_indices - 2)];

        let v0 = (transform * TRACTOGRAM_VERTICES[index    ]).xyz;
        let v1 = (transform * TRACTOGRAM_VERTICES[index + 1]).xyz;

        if (ENVIRONMENT.settings.quality == 1u) {
            voxelize_sdf(v0, v1, radius);
        } else {
            voxelize(v0, v1, radius);
        }
    }
}

fn voxelize_sdf(v0: vec3<f32>, v1: vec3<f32>, radius: f32) {
    let delta = v1 - v0;
    let direction = normalize(delta);
    let rank = rank_axes(abs(delta));

    let v0_radius = v0 - direction * radius;
    let v1_radius = v1 + direction * radius;
    let delta_radius = v1_radius - v0_radius;
    let distance_radius = length(delta_radius);

    let step = abs(delta_radius / delta_radius[rank[0]]);
    let step_length = length(step);
    let width = 0.5 * step + radius;

    let minusplus = vec2<f32>(-1.0, 1.0);

    for (var distance = 0.5 * step_length; distance <= distance_radius; distance += step_length) {
        let position = v0 + distance * direction;

        let axis_0 = u32(position[rank[0]]);

        let bounds_1 = vec2<u32>(position[rank[1]] + minusplus * width[rank[1]]);
        let bounds_2 = vec2<u32>(position[rank[2]] + minusplus * width[rank[2]]);

        for (var axis_1 = bounds_1.x; axis_1 <= bounds_1.y; axis_1++) {
            for (var axis_2 = bounds_2.x; axis_2 <= bounds_2.y; axis_2++) {
                var voxel = vec3<u32>();
                voxel[rank[0]] = axis_0;
                voxel[rank[1]] = axis_1;
                voxel[rank[2]] = axis_2;

                let coverage = aaa(vec3<f32>(voxel) + 0.5, v0, v1, radius);
                let coverage_u32 = u32(coverage * f32(U24_MAX));

                if (coverage_u32 > 0) {
                    atomicAdd(&DENSITY[linear_index(voxel)], coverage_u32);
                }
            }
        }
    }
}

fn rank_axes(v: vec3<f32>) -> vec3<u32> {
    var val = v;
    var idx = vec3<u32>(0u, 1u, 2u);

    if (val.x < val.y) {
        val = val.yxz;
        idx = idx.yxz;
    }
    if (val.y < val.z) {
        val = val.xzy;
        idx = idx.xzy;
    }
    if (val.x < val.y) {
        val = val.yxz;
        idx = idx.yxz;
    }

    return idx;
}

fn aaa(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let r_clamped = max(r, 0.5);
    let distance = saturate(0.5 - capsule(p, a, b, r_clamped)) - saturate(0.5 - sphere(p - a, r_clamped));
    return distance * saturate(r);
}

fn capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

fn sphere(p: vec3<f32>, r: f32) -> f32 {
  return length(p) - r;
}

fn voxelize(v0: vec3<f32>, v1: vec3<f32>, radius: f32) {
    let area = PI * radius * radius * f32(U24_MAX);

    let delta = v1 - v0;
    let distance = length(delta);
    let direction = delta / distance;
    let voxel_boundaries = 1.0 / abs(direction);
    let step = vec3<i32>(sign(direction));
    var next = vec4<f32>(
        one_if_zero(abs(fract(vec3<f32>(-step) * fract(v0)))) * voxel_boundaries,
        distance
    );

    var voxel = vec3<i32>(v0);

    while (next.w > 0.0) {
        let increment = minimum(next);

        let idx = linear_index(vec3<u32>(voxel));

        atomicAdd(&DENSITY[idx], u32(area * increment));

        let mask = next == vec4<f32>(increment);
        voxel += select(vec3<i32>(0), step, mask.xyz);
        next = select(
            next - increment,
            vec4<f32>(voxel_boundaries, 0.0),
            mask
        );
    }
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}
