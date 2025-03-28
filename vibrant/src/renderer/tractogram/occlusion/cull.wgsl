@group(0) @binding(0) var OCCLUSION: texture_2d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(1) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;
@group(1) @binding(2) var<storage> TRACTOGRAM_VERTICES: array<vec4<f32>>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var<storage, read_write> TRACTOGRAM_INDICES: array<u32>;
@group(3) @binding(1) var<storage, read_write> OFFSET: atomic<u32>;

const CHUNK_SIZE: u32 = 32;

@compute
@workgroup_size(1024)
fn main(@builtin(global_invocation_id) id: vec3<u32>)
{
    for (var i=0u; i<CHUNK_SIZE; i++) {
        let index = id.x * CHUNK_SIZE + i;

        if (index >= arrayLength(&TRACTOGRAM_INDICES)) { return; }

        if (should_keep(index)) {
            TRACTOGRAM_INDICES[atomicAdd(&OFFSET, 1u)] = index;
        }
    }
}

fn should_keep(index: u32) -> bool {
    let dim = vec2<f32>(textureDimensions(OCCLUSION));

    let v0 = TRACTOGRAM_VERTICES[index + 0];
    let v1 = TRACTOGRAM_VERTICES[index + 1];

    // Next Streamline Marker Vertex Culling
    if (v0.w == 0.0 || v1.w == 0.0) { return false; }

    let transform = ENVIRONMENT.camera.projection * TRACTOGRAM_TO_WORLD;

    var v0_clip = transform * v0; v0_clip /= v0_clip.w;
    var v1_clip = transform * v1; v1_clip /= v1_clip.w;

    // Frustum culling
    if (!(all(abs(v0_clip.xy) < vec2<f32>(1.0)) || all(abs(v1_clip.xy) < vec2<f32>(1.0)))) { return false; }

    let vmin = min(v0_clip.xy, v1_clip.xy);
    let vmax = max(v0_clip.xy, v1_clip.xy);
    let vdim = (vmax - vmin) * dim;

    let centroid = 0.5 * (vmin + vmax);
    let sample = 0.5 * centroid + 0.5;

    let level = max(0.0, log2(max(vdim.x, vdim.y)));

    let max_depth = textureSampleLevel(OCCLUSION, SAMPLER, sample, level).x;

    // Occlusion Culling
    return min(v0_clip.z, v1_clip.z) < max_depth;
}
