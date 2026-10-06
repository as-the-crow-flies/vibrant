struct CascadeOpts {
    include_volume: u32,
    include_lines: u32,
}

@group(0) @binding(0) var PHI_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(1) var VMM_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var PHI_IN: texture_3d<f32>;
@group(0) @binding(4) var VMM_IN: texture_3d<f32>;
@group(0) @binding(5) var IRRADIANCE_OUT: texture_storage_3d<rgba16float, write>;
@group(0) @binding(6) var IRRADIANCE_IN: texture_3d<f32>;
@group(0) @binding(8) var<uniform> CASCADE_OPTS: CascadeOpts;

// Diagnostic Histogram
@group(0) @binding(7) var<storage, read_write> EM_ITERATIONS: array<atomic<u32>>;

@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var PROPERTIES: texture_3d<f32>;
@group(1) @binding(4) var GRADIENT: texture_3d<f32>;
@group(1) @binding(5) var SAMPLER: sampler;

@group(1) @binding(8) var LINE_EXTINCTION: texture_3d<f32>;
@group(1) @binding(9) var LINE_EXTINCTION_SAMPLER: sampler;
@group(1) @binding(6) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(7) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;
@group(3) @binding(3) var<storage, read> VMM_HDRI: array<vec4<f32>>;
@group(3) @binding(4) var<storage, read> PHI_HDRI: array<vec4<f32>>;

// LEVEL SAMPLES  WORKGROUP  SUBGROUPS  SAMPLES_PER_THREAD
//     0      32         32          1                   1
//     1      32         32          1                   1
//     2     128         32          1                   4
//     3     256         64          2                   4
//     4    1024        256          8                   4
//     5    4096       1024         32                   4

const CASCADE_MAX: u32 = 5u;
const CASCADE: u32 = #CASCADE;
const WORKGROUP: u32 = #WORKGROUP;
const SUBGROUPS: u32 = WORKGROUP / 32u;
const SAMPLES: u32 = WORKGROUP * SAMPLES_PER_THREAD;
const EM_HISTOGRAM_ROW: u32 = CASCADE;

const INTERVAL = array<f32, 7>(1.0, 3.0, 7.0, 15.0, 31.0, 63.0, 127.0);

const IRRADIANCE_CULL: f32 = 0.05;

// Each sample covers a solid angle of 4π / SAMPLES ≈ π tan²θ.
const CONE_TAN: f32 = 2.0 / sqrt(f32(SAMPLES));

var<workgroup> CULL_OCCUPIED: atomic<u32>;
var<workgroup> CULL_RESULT: u32;

const CULL_NEIGHBORS: u32 = 6u;
const NEIGHBOR_OFFSET = array<vec3<f32>, 6>(
    vec3<f32>(-1.0,  0.0,  0.0), vec3<f32>( 1.0,  0.0,  0.0),
    vec3<f32>( 0.0, -1.0,  0.0), vec3<f32>( 0.0,  1.0,  0.0),
    vec3<f32>( 0.0,  0.0, -1.0), vec3<f32>( 0.0,  0.0,  1.0),
);

@compute
@workgroup_size(WORKGROUP)
fn main(@builtin(workgroup_id) probe: vec3<u32>, thread: Thread) {
    if (thread.index < VMM_SIZE) { initialize_lobe(probe, thread.index); }

    workgroupBarrier();

    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    if (is_culled(origin, thread)) {
        if (thread.index < VMM_SIZE) { store_lobe(probe, thread.index); }
        if (thread.index == 0u) { record_iterations(0); }
        return;
    }

    let omega = hammersley_directions(thread);
    let radiance = incident_radiance(origin, omega);
    let weight = radiance_weights(radiance);

    fit_vmm(omega, radiance, weight, thread);

    if (thread.index < VMM_SIZE) { store_lobe(probe, thread.index); }
    if (thread.index == 0u) { store_irradiance(probe); }
}

fn is_culled(origin: vec3<f32>, thread: Thread) -> bool {
    let dim = vec3<f32>(probe_grid(textureDimensions(VMM_OUT))) * f32(1u << CASCADE);
    let origin_sample = origin / dim;
    let cell_sample = f32(1u << CASCADE) / dim;

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;
    let mip = f32(CASCADE) + log2(radiance_scale);

    if (thread.index == 0u) { atomicStore(&CULL_OCCUPIED, 0u); }

    workgroupBarrier();

    if (thread.index < CULL_NEIGHBORS) {
        let sample = origin_sample + NEIGHBOR_OFFSET[thread.index] * cell_sample;
        let extinction = extinction_at(sample, mip);
        if (any(extinction > vec3<f32>(EPSILON))) { atomicOr(&CULL_OCCUPIED, 1u); }
    }

    workgroupBarrier();

    var result = atomicLoad(&CULL_OCCUPIED) == 0u;
    if (!result && CASCADE < CASCADE_MAX) {
        let irradiance = unpack_rgb(textureSampleLevel(IRRADIANCE_IN, SAMPLER, origin_sample, 0.0));
        result = brightness(irradiance) < IRRADIANCE_CULL;
    }

    if (thread.index == 0u) { CULL_RESULT = select(0u, 1u, result); }

    return workgroupUniformLoad(&CULL_RESULT) != 0u;
}

fn initialize_lobe(probe: vec3<u32>, k: u32) {
    if (CASCADE == CASCADE_MAX) {
        let vmm = VMM_HDRI[VMM_SIZE + k];
        let phi = PHI_HDRI[VMM_SIZE + k];

        VMM[k] = vec4<f32>(rotate_hdri(vmm.xyz) * length(vmm.xyz), vmm.w);
        PHI[k] = vec4<f32>(phi.rgb * HDRI_SETTINGS.strength, phi.w);
    } else {
        let uv = parent_uv(textureDimensions(VMM_IN), probe, k);

        VMM[k] = textureSampleLevel(VMM_IN, SAMPLER, uv, 0.0);
        PHI[k] = textureSampleLevel(PHI_IN, SAMPLER, uv, 0.0);
    }

    VMM_PRIOR[k] = VMM[k];
}

// The HDRI fit is baked in world space: rotate it, then bring it into the volume.
fn rotate_hdri(direction: vec3<f32>) -> vec3<f32> {
    let world = rotation_z(-HDRI_SETTINGS.rotation * 2.0 * PI) * normalize(direction);
    return normalize((TRANSFORM * vec4<f32>(world, 0.0)).xyz);
}

fn store_lobe(probe: vec3<u32>, k: u32) {
    let texel = probe + probe_grid(textureDimensions(VMM_OUT)) * lobe_tile(k);

    let vmm = VMM[k];
    let phi = mean_radiance(PHI[k]);

    textureStore(VMM_OUT, texel, vmm);
    textureStore(PHI_OUT, texel, phi);
}

fn store_irradiance(probe: vec3<u32>) {
    var irradiance = vec3<f32>(0.0);

    for (var k=0u; k<VMM_SIZE; k++) {
        irradiance += mean_radiance(PHI[k]).rgb;
    }

    textureStore(IRRADIANCE_OUT, probe, pack_rgb(irradiance));
}

fn incident_radiance(origin: vec3<f32>, omega: Directions) -> Radiances {
    var radiance = Radiances();
    var expectation_sum = Weights();

    for (var k=0u; k<VMM_SIZE; k++) {
        if (PHI[k].w < EPSILON) { continue; }

        let expectation = vmf(VMM[k], omega);
        expectation_sum += expectation;

        radiance += outer(mean_radiance(PHI[k]).rgb, expectation);
    }

    radiance = scale(radiance, 1.0 / expectation_sum);

    return hadamard(radiance, trace_samples(origin, omega));
}

fn trace(origin: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    return transmission(origin, direction, INTERVAL[CASCADE], INTERVAL[CASCADE + 1]);
}

fn transmission(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> vec3<f32> {
    var transmission = vec3<f32>(1.0);

    let dim = vec3<f32>(probe_grid(textureDimensions(VMM_OUT))) * f32(1u << CASCADE);
    let origin_sample = origin / dim;
    let direction_sample = direction / dim;

    let radiance_scale = f32(textureDimensions(EXTINCTION).x) / dim.x;

    let scale = length(TRANSFORM[0].xyz) * dim.x; // voxels/mm
    let voxel = 1.0 / radiance_scale;

    var t = t0;

    while (t < t1) {
        let diameter = max(2.0 * CONE_TAN * t, voxel);
        let step = min(diameter, t1 - t);
        let sample = origin_sample + direction_sample * (t + 0.5 * step);

        if (any(abs(sample - 0.5) > vec3<f32>(0.5))) { break; }

        let extinction = extinction_at(sample, log2(diameter / voxel));

        transmission *= exp(-extinction * step / scale);

        if (all(transmission < vec3<f32>(1e-3))) { break; }

        t += step;
    }

    return transmission;
}

// Occlusion extinction for the cascade: the volume's `EXTINCTION` and/or the
// line deposit's `LINE_EXTINCTION`, per this radiance buffer's `cascade_opts`.
fn extinction_at(uv: vec3<f32>, mip: f32) -> vec3<f32> {
    var e = vec3<f32>(0.0);
    if (CASCADE_OPTS.include_volume != 0u) {
        e += unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, uv, mip));
    }
    if (CASCADE_OPTS.include_lines != 0u) {
        e += vec3<f32>(textureSampleLevel(LINE_EXTINCTION, LINE_EXTINCTION_SAMPLER, uv, mip).x);
    }
    return e;
}

fn parent_uv(dim: vec3<u32>, probe: vec3<u32>, k: u32) -> vec3<f32> {
    let half = vec3<f32>(0.5);
    let parent_grid = vec3<f32>(probe_grid(dim));
    let parent_probe = clamp(half * (vec3<f32>(probe) + half), half, parent_grid - half);
    return (parent_probe + parent_grid * vec3<f32>(lobe_tile(k))) / vec3<f32>(dim);
}

fn probe_grid(dim: vec3<u32>) -> vec3<u32> {
    return max(dim / vec3<u32>(4, 4, 2), vec3<u32>(1));
}

fn lobe_tile(k: u32) -> vec3<u32> {
    return vec3<u32>(k & 3, (k >> 2) & 3, (k >> 4) & 1);
}

fn mean_radiance(v: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(v.rgb / max(v.w, EPSILON), 1.0);
}
