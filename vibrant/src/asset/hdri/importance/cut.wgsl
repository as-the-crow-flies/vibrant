@group(0) @binding(0) var IMPORTANCE: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> CUT: array<vec4<f32>>;

const LOBES: u32 = #LOBES;

// Smallest split whose lobe stays within the κ clamp, as a fraction of 4π.
const SOLID_ANGLE_MIN: f32 = 1.0 - 1.0 / U_MIN;

// rect = (x, y, w, h) in level 0 texels, sum = Σ (Y·ω, Y) over the rect.
struct Node {
    rect: vec4<u32>,
    sum: vec4<f32>,
}

var<private> NODES: array<Node, LOBES>;
var<private> SIZE: u32;

// Greedy binary cut: split the most powerful node in half, alternating x and y.
// Every power-of-two cut K is a prefix of the greedy order and lands in CUT[K..2K].
@compute
@workgroup_size(1)
fn main() {
    SIZE = textureDimensions(IMPORTANCE).x;
    NODES[0] = node(vec4<u32>(0u, 0u, SIZE, SIZE));

    for (var count = 1u; ; count++) {
        if (countOneBits(count) == 1u) { store_cut(count); }
        if (count == LOBES) { break; }

        split(most_powerful(count), count);
    }
}

fn most_powerful(count: u32) -> u32 {
    var best = 0u;
    var power = -1.0;

    for (var k = 0u; k < count; k++) {
        if (is_splittable(NODES[k].rect) && NODES[k].sum.w > power) {
            best = k;
            power = NODES[k].sum.w;
        }
    }

    return best;
}

fn is_splittable(rect: vec4<u32>) -> bool {
    return 0.5 * solid_angle(rect) >= SOLID_ANGLE_MIN;
}

fn split(k: u32, count: u32) {
    let rect = NODES[k].rect;

    var half = rect;
    var offset = vec2<u32>(0u);

    if (rect.z == rect.w) {
        half.z /= 2u;
        offset.x = half.z;
    } else {
        half.w /= 2u;
        offset.y = half.w;
    }

    NODES[k] = node(half);
    NODES[count] = node(half + vec4<u32>(offset, 0u, 0u));
}

fn node(rect: vec4<u32>) -> Node {
    let m = min(rect.z, rect.w);
    let level = firstTrailingBit(m);
    let p = rect.xy / m;

    var sum = textureLoad(IMPORTANCE, p, level);
    if (rect.z != rect.w) { sum += textureLoad(IMPORTANCE, p + rect.zw / m - 1u, level); }

    return Node(rect, sum * f32(m * m));
}

fn store_cut(count: u32) {
    for (var k = 0u; k < count; k++) {
        CUT[count + k] = lobe(NODES[k], count);
    }
}

// Moment match a uniform cap of the node's solid angle Ω: r = 1 − Ω/4π.
fn lobe(node: Node, count: u32) -> vec4<f32> {
    let direction = select(center(node.rect), normalize(node.sum.xyz), length(node.sum.xyz) > 0.0);
    let r = 1.0 - solid_angle(node.rect);

    return vec4<f32>(direction * r, 1.0) / f32(count);
}

fn center(rect: vec4<u32>) -> vec3<f32> {
    let uv = 2.0 * (vec2<f32>(rect.xy) + 0.5 * vec2<f32>(rect.zw)) / f32(SIZE) - 1.0;
    return clarberg_equal_area_sphere(uv);
}

// As a fraction of 4π.
fn solid_angle(rect: vec4<u32>) -> f32 {
    return f32(rect.z * rect.w) / f32(SIZE * SIZE);
}
