@group(0) @binding(0) var HIZ: texture_2d<f32>;
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
    let dim = vec2<f32>(textureDimensions(HIZ));

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

    let level = min(u32(ceil(max(0.0, log2(max(vdim.x, vdim.y))))), textureNumLevels(HIZ) - 1u);
    let surface_dim = vec2<f32>(ENVIRONMENT.surface);
    let hiz_dim = vec2<f32>(textureDimensions(HIZ, i32(level)) * (ENVIRONMENT.tile << level));

    let max_depth = textureSampleLevel(HIZ, SAMPLER, surface_dim / hiz_dim * sample, f32(level)).x;

    // Occlusion Culling
    return linearize_depth(min(v0_clip.z, v1_clip.z)) <= max_depth;
}

fn linearize_depth(ndc_depth: f32) -> f32 {
    let near = ENVIRONMENT.camera.near;
    let far = ENVIRONMENT.camera.far;

    // Convert NDC depth [0, 1] to clip-space Z [-1, 1]
    let z = ndc_depth * 2.0 - 1.0;

    // Reverse the projection to get view-space Z
    let view_z = (2.0 * near * far) / (far + near - z * (far - near));

    // Convert view-space Z to linear depth in [0, 1]
    return (view_z - near) / (far - near);
}
