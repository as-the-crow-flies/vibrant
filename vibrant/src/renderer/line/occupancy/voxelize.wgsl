@group(0) @binding(0) var<storage, read_write> DENSITY: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> TANGENT: array<atomic<u32>>;

@group(1) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(1) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read_write> LINE_COUNT: atomic<u32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

const WORKGROUP_SIZE: u32 = 1024;
const CHUNK_SIZE: u32 = 32;

const PI: f32 = 3.14159265358979323846264338327950288;

var<workgroup> OFFSET: u32;

var<private> RADIUS: f32;
var<private> DENSITY_MULTIPLIER: f32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(local_invocation_index) local: u32) {
    let n_indices = arrayLength(&LINE_INDEX);

    RADIUS = ENVIRONMENT.settings.streamline_radius;
    DENSITY_MULTIPLIER = PI * RADIUS * RADIUS;

    var offset = 0u;

    while (offset < n_indices) {
        if (local == 0) {
            OFFSET = atomicAdd(&LINE_COUNT, CHUNK_SIZE * WORKGROUP_SIZE);
        }

        offset = workgroupUniformLoad(&OFFSET);

        for (var i = 0u; i < CHUNK_SIZE; i++) {
            let index_index = offset + i * WORKGROUP_SIZE + local;

            if (index_index >= n_indices) { continue; }

            let index = LINE_INDEX[index_index];
            let v0 = unpack_vertex(LINE_VERTEX[index + 0]);
            let v1 = unpack_vertex(LINE_VERTEX[index + 1]);

            voxelize(index, v0, v1, RADIUS);
        }
    }
}

fn visit_voxel_line(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex, length: f32) {
    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));
    let density = 2.0 * DENSITY_MULTIPLIER * length;
    let tangent = density * normalize(abs(v1.xyz - v0.xyz));

    atomicAdd(&DENSITY[idx], encode_density(density));
    atomicAdd(&TANGENT[idx], encode_tangent(tangent));
}

fn visit_voxel(voxel: vec3<i32>, index: u32, v0: Vertex, v1: Vertex) {
    let smoothing = ENVIRONMENT.settings.smoothing;

    let radius_clamp = max(smoothing, RADIUS);
    let radius_ratio = RADIUS / radius_clamp;

    let idx = block_index(vec3<u32>(voxel), vec3<u32>(ENVIRONMENT.volume));

    let sample = vec3<f32>(voxel) + 0.5;

    let sample_v0 = sample - v0.xyz;
    let delta = v1.xyz - v0.xyz;
    let height = clamp(dot(sample_v0, delta) / dot(delta, delta), 0.0, 1.0);

    let sdf = cylinder(sample, v0.xyz, v1.xyz, radius_clamp);

    let density = radius_ratio * mix(v0.alpha, v1.alpha, height) * saturate(0.5 - sdf);
    let tangent = density * abs(normalize(delta));

    atomicAdd(&DENSITY[idx], encode_density(density));
    atomicAdd(&TANGENT[idx], encode_tangent(tangent));
}

fn encode_density(density: f32) -> u32 {
    return (u32(density * U12_MAX_f32) << U14_SHIFT) + 1;
}

fn encode_tangent(tangent: vec3<f32>) -> u32 {
    let tangent_encoded = vec3<u32>(tangent * U12_MAX_f32);
    return tangent_encoded.x << 16 | tangent_encoded.y;
}

fn cylinder(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
  let ba = b - a;
  let pa = p - a;
  let baba = dot(ba,ba);
  let paba = dot(pa,ba);
  let x = length(pa*baba-ba*paba) - r*baba;
  let y = abs(paba-baba*0.5) - baba*0.5;
  let x2 = x*x;
  let y2 = y*y*baba;
  let d = select(
    select(0.0, x2, x>0.0) + select(0.0, y2, y>0.0),
    -min(x2,y2),
    max(x,y) < 0.0
 );
  return sign(d) * sqrt(abs(d)) / baba;
}
