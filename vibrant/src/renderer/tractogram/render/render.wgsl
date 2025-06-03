@group(0) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(0) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(1) @binding(0) var<storage> OFFSET: array<u32>;
@group(1) @binding(2) var<storage> INDEX: array<u32>;

@group(2) @binding(0) var COUNT: texture_3d<u32>;

@group(3) @binding(0) var DENSITY: texture_3d<f32>;
@group(3) @binding(1) var SAMPLER: sampler;

@group(4) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(5) @binding(0) var COLOR: texture_storage_2d<bgra8unorm, write>;

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel = id.xy;

    if (any(pixel >= ENVIRONMENT.surface)) { return; }

    let result = compute(pixel);

    textureStore(COLOR, vec2<u32>(pixel.x, ENVIRONMENT.surface.y - pixel.y), result);
}

fn compute(pixel: vec2<u32>) -> vec4<f32> {
    let dim_u32 = vec3<u32>(textureDimensions(DENSITY));
    let dim = vec3<f32>(dim_u32);

    let radius = ENVIRONMENT.settings.streamline_radius / dim.x;

    let uv = vec2<f32>(pixel) / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0;

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));

    let origin = near;
    let direction = normalize(far - near);

    let hit = aabb(origin, direction, vec3<f32>(0.5));
    let distance = hit.y - hit.x;

    if (distance <= 0) { return vec4<f32>(); }

    let t0 = (origin + direction * 1.001 * hit.x + 0.5) * dim;

    let voxel_boundaries = vec4<f32>(1.0 / abs(direction), 0.0);
    let step = vec3<i32>(sign(direction));
    var next = vec4<f32>(
        one_if_zero(fract(abs(sign(direction) - t0 + floor(t0)))) * voxel_boundaries.xyz,
        distance * dim.x
    );

    var voxel = vec3<i32>(floor(t0));
    var color = vec4<f32>(0.0);

    while (next.w > 0.0) {
        let increment = minimum4(next);

        var closest = 10.0;
        var color = vec3<f32>(0.0);

        if (all(abs(voxel) < vec3<i32>(dim_u32))) {
            let count = textureLoad(COUNT, voxel, 0).x;
            let offset = OFFSET[block_index(vec3<u32>(voxel), dim_u32)] - count;

            for (var i = 0u; i < count; i++) {
                let index = INDEX[offset + i];
                let v0 = TRACTOGRAM_TO_WORLD * TRACTOGRAM_VERTICES[index + 0];
                let v1 = TRACTOGRAM_TO_WORLD * TRACTOGRAM_VERTICES[index + 1];

                let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, radius);

                let position = origin + hit * direction;
                let in_voxel = all(vec3<i32>((position + 0.5) * dim) == voxel);

                if (hit < closest && in_voxel) {
                    closest = hit;
                    color = capsule_normal(position, v0.xyz, v1.xyz, radius) * 0.5 + 0.5;
                }
            }
        }

        if (closest < 10.0) {
            return vec4<f32>(color, 1.0);
        }

        let mask = next == vec4<f32>(increment);
        voxel = voxel + step * vec3<i32>(mask.xyz);
        next = select(next - increment, voxel_boundaries, mask);
    }

    return color;
}

fn aabb(origin: vec3<f32>, direction: vec3<f32>, size: vec3<f32>) -> vec2<f32>
{
    let m = 1.0 / direction;
    let n = m * origin;
    let k = abs(m) * size;
    let t1 = -n - k;
    let t2 = -n + k;
    let tN = max(maximum(t1), 0.0);
    let tF = minimum(t2);
    if( tN>tF || tF<0.0) { return vec2(-1.0); } // no intersection
    return vec2<f32>( tN, tF );
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn minimum4(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v < vec3<f32>(1E-6));
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}

fn sdCapsule(p: vec3<f32>,a: vec3<f32>,b: vec3<f32>, r: f32) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba*h) - r;
}

// https://iquilezles.org/articles/intersectors
fn capsule_intersection(ro: vec3<f32>, rd: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> f32
{
    let ba = pb - pa;
    let oa = ro - pa;

    let baba = dot(ba,ba);
    let bard = dot(ba,rd);
    let baoa = dot(ba,oa);
    let rdoa = dot(rd,oa);
    let oaoa = dot(oa,oa);

    var a = baba      - bard*bard;
    var b = baba*rdoa - baoa*bard;
    var c = baba*oaoa - baoa*baoa - r*r*baba;
    var h = b*b - a*c;

    if (h>=0.0)
    {
        let t = (-b-sqrt(h))/a;
        let y = baoa + t*bard;

        // body
        if( y>0.0 && y<baba ) { return t; }

        // caps
        let oc = select(ro - pb, oa, y <= 0.0);

        b = dot(rd, oc);
        c = dot(oc, oc) - r*r;
        h = b*b - c;
        if (h>0.0) { return -b - sqrt(h); }
    }

    return 1E6;
}

fn capsule_normal(pos: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> vec3<f32>
{
    let ba = b - a;
    let pa = pos - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return (pa - h*ba) / r;
}
