struct KBufferItem {
    depth: f32,
    color: u32
}

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(2) @interpolate(flat) v0: vec4<f32>,
    @location(3) @interpolate(flat) v1: vec4<f32>,
}

@group(0) @binding(0) var<storage> LINE_INDEX: array<u32>;
@group(0) @binding(1) var<storage> LINE_VERTEX: array<vec4<f32>>;
@group(0) @binding(4) var<storage> LINE_CULL: array<u32>;

@group(1) @binding(0) var<storage, read_write> KBUFFER: array<KBufferItem>;
@group(1) @binding(1) var<storage, read_write> LOCK: array<atomic<u32>>;

@group(2) @binding(1) var SAMPLER: sampler;
@group(2) @binding(4) var OCCLUSION_AMBIENT: texture_3d<f32>;
@group(2) @binding(6) var OCCLUSION_DIRECTIONAL: texture_3d<f32>;
@group(2) @binding(8) var OPACITY: texture_2d<f32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const QUAD_INDEX: array<u32, 6> = array<u32, 6>(0, 1, 2, 1, 2, 3);

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> Fragment {
    let scale = 1.0 / f32(ENVIRONMENT.volume);
    let radius = ENVIRONMENT.settings.radius * scale;

    // if (bool(LINE_CULL[instance_index])) { return Fragment(); }

    let index = LINE_INDEX[instance_index];

    let v0 = LINE_VERTEX[index + 0];
    let v1 = LINE_VERTEX[index + 1];

    let eye = ENVIRONMENT.camera.transform[3].xyz;
    let view = normalize(-ENVIRONMENT.camera.transform[2].xyz);
    let quad = generate_aligned_box_billboard(eye, v0.xyz, v1.xyz, radius);

    let position = quad[QUAD_INDEX[vertex_index]].xyz;
    let clip = ENVIRONMENT.camera.projection * vec4<f32>(position, 1.0);

    return Fragment(clip, position, v0, v1);
}

@fragment
fn fragment(fragment: Fragment) -> @location(0) vec4<f32> {
    let pixel = vec2<u32>(fragment.clip.xy);
    let pixel_index = pixel.y * ENVIRONMENT.surface.x + pixel.x;

    // if (textureLoad(OPACITY, pixel >> vec2<u32>(1), 0).x > MAX_OPACITY) { discard; }

    let radius = ENVIRONMENT.settings.radius / f32(ENVIRONMENT.volume);

    let origin = ENVIRONMENT.camera.transform[3].xyz;
    let direction = normalize(fragment.position - origin);

    let v0 = unpack_vertex(fragment.v0);
    let v1 = unpack_vertex(fragment.v1);

    let hit = capsule_intersection(origin, direction, v0.xyz, v1.xyz, radius);

    let position = origin + hit * direction;

    if (hit == 1E6 || should_be_clipped(v0, v1, position)) { discard; }

    let color = shade(v0, v1, radius, position, ENVIRONMENT, OCCLUSION_AMBIENT, OCCLUSION_DIRECTIONAL, SAMPLER);

    var current = KBufferItem(hit, pack_color_kbuffer(color));

    if (try_lock(pixel_index)) {
        for (var k=0u; k<K; k++) {
            let index = pixel_index * K + k;

            let previous = KBUFFER[index];

            if (current.depth > previous.depth) {
                KBUFFER[index] = current;
                current = previous;
            }
        }

        release_lock(pixel_index);
    }

    return unpack_color_kbuffer(current.color);
}

fn try_lock(index: u32) -> bool {
    for (var i = 0u; i < 32; i++) {
        if (atomicExchange(&LOCK[index], 1u) == 0u) {
            return true;
        }
    }

    return true;
}

fn release_lock(index: u32) {
    atomicStore(&LOCK[index], 0u);
}
