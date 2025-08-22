struct Push {
    start: u32,
    end: u32
}

@group(0) @binding(0) var<storage, read_write> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage, read_write> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> LINE_CULL: array<u32>;

@group(1) @binding(0) var HIZ: texture_2d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

@group(2) @binding(0) var<uniform> PUSH: Push;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(1024)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let instance_index = id.x + PUSH.start;

    if (instance_index > PUSH.end) { return; }

    let scale = 1.0 / f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius * scale;

    let index = LINE_INDEX[instance_index];

    let v0 = LINE_VERTEX[index + 0];
    let v1 = LINE_VERTEX[index + 1];

    let eye = ENVIRONMENT.camera.transform[3].xyz;
    let view = normalize(-ENVIRONMENT.camera.transform[2].xyz);
    let quad = generate_aligned_box_billboard(eye, v0.xyz, v1.xyz, radius);

    let p00 = project(quad[0]);
    let p10 = project(quad[1]);
    let p01 = project(quad[2]);
    let p11 = project(quad[3]);

    let delta = abs(p11 - p00);
    let level = u32(log2(max(delta.x, delta.y)));

    let c00 = can_be_culled(p00, level);
    let c01 = can_be_culled(p01, level);
    let c10 = can_be_culled(p10, level);
    let c11 = can_be_culled(p11, level);

    LINE_CULL[instance_index] = u32(c00 && c01 && c10 && c11);
}

fn can_be_culled(sample: vec3<f32>, level: u32) -> bool {
    let s = 0.5 + 0.5 * vec2<f32>(1.0, -1.0) * sample.xy;
    return textureSampleLevel(HIZ, SAMPLER, s, f32(level)).x < sample.z;
}

fn project(vector: vec4<f32>) -> vec3<f32> {
    let p = ENVIRONMENT.camera.projection * vector;
    return p.xyz / p.w;
}
