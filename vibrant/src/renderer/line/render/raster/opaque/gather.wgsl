struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) @interpolate(flat) index: u32,
    @location(2) @interpolate(flat) v0: vec3<f32>,
    @location(3) @interpolate(flat) v1: vec3<f32>,
}

struct Visibility {
    @location(0) index: u32,
    @builtin(frag_depth) depth: f32,
}

@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(3) var<storage> LINE_CULL: array<u32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

const QUAD_INDEX: array<u32, 6> = array<u32, 6>(0, 1, 2, 1, 2, 3);

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> Fragment {
    let scale = 1.0 / f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius * scale;

    // let culled = bool(LINE_CULL[instance_index]);

    // if (culled) { return Fragment(); }

    let index = LINE_INDEX[instance_index];

    let v0 = LINE_VERTEX[index + 0];
    let v1 = LINE_VERTEX[index + 1];

    let v0s = v0.xyz * scale - 0.5;
    let v1s = v1.xyz * scale - 0.5;

    let eye = ENVIRONMENT.camera.transform[3].xyz;
    let view = normalize(-ENVIRONMENT.camera.transform[2].xyz);
    let quad = generate_aligned_box_billboard(eye, v0s, v1s, radius);

    let position = quad[QUAD_INDEX[vertex_index]].xyz;
    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);

    return Fragment(clip, position, index, v0s, v1s);
}

@fragment
fn fragment(fragment: Fragment) -> Visibility {
    let pixel = vec2<u32>(fragment.clip.xy);
    let pixel_index = pixel.y * ENVIRONMENT.surface.x + pixel.x;

    // if (textureLoad(OPACITY, pixel >> vec2<u32>(1), 0).x > MAX_OPACITY) { discard; }

    let scale = 1.0 / f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius * scale;

    let origin = ENVIRONMENT.camera.transform[3].xyz;
    let direction = normalize(fragment.position - origin);

    let hit = capsule_intersection(origin, direction, fragment.v0, fragment.v1, radius);

    if (hit == 1E6) { discard; }

    let position = origin + hit * direction;

    // if (should_be_clipped(fragment.v0, fragment.v1, position)) { discard; }

    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);
    let depth = clip.z / clip.w;

    return Visibility(fragment.index, depth);
}
