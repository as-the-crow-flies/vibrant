@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var SAMPLER: sampler;

@group(1) @binding(0) var CULLING: texture_3d<f32>;

@group(2) @binding(0) var AMBIENT: texture_storage_3d<r32float, read_write>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const ICOSAHEDRON_ONE: f32 = 0.8506508;
const ICOSAHEDRON_PHI: f32 = 0.5257311;

const ICOSAHEDRON: array<vec3<f32>, 12> = array<vec3<f32>, 12>(
    vec3<f32>(0.0,  ICOSAHEDRON_ONE,  ICOSAHEDRON_PHI),
    vec3<f32>(0.0,  ICOSAHEDRON_ONE, -ICOSAHEDRON_PHI),
    vec3<f32>(0.0, -ICOSAHEDRON_ONE,  ICOSAHEDRON_PHI),
    vec3<f32>(0.0, -ICOSAHEDRON_ONE, -ICOSAHEDRON_PHI),

    vec3<f32>( ICOSAHEDRON_ONE,  ICOSAHEDRON_PHI, 0.0),
    vec3<f32>( ICOSAHEDRON_ONE, -ICOSAHEDRON_PHI, 0.0),
    vec3<f32>(-ICOSAHEDRON_ONE,  ICOSAHEDRON_PHI, 0.0),
    vec3<f32>(-ICOSAHEDRON_ONE, -ICOSAHEDRON_PHI, 0.0),

    vec3<f32>( ICOSAHEDRON_PHI, 0.0,  ICOSAHEDRON_ONE),
    vec3<f32>(-ICOSAHEDRON_PHI, 0.0,  ICOSAHEDRON_ONE),
    vec3<f32>( ICOSAHEDRON_PHI, 0.0, -ICOSAHEDRON_ONE),
    vec3<f32>(-ICOSAHEDRON_PHI, 0.0, -ICOSAHEDRON_ONE));

const ONE_OVER_TWELVE: f32 = 1.0 / 12.0;
const TAN_CONE_ANGLE: f32 = 1.6403417719345383;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    let dim = f32(textureDimensions(AMBIENT).x);
    let one_over_dim = 1.0 / dim;
    let one_over_alpha = 1.0 / ENVIRONMENT.settings.alpha;

    let position = vec3<f32>(voxel) + 0.5;

    var total = 0.0;

    if (culling(position * one_over_dim, 1.0) > 0.0) {
        for (var i=0u; i<12; i++) {
            var direction = ICOSAHEDRON[i];

            var occlusion = 0.0;

            for (var distance = 1.0; distance < dim; distance *= 2.0) {
                let sample = (position + direction * distance) * one_over_dim;
                let level = log2(TAN_CONE_ANGLE * distance);

                occlusion += (1.0 - occlusion) * density(sample, level);

                if (occlusion > 0.99 || any(abs(sample - 0.5) >= vec3<f32>(0.5))) { break; }
            }

            total += occlusion;
        }

        textureStore(AMBIENT, voxel, vec4<f32>(total * ONE_OVER_TWELVE));
    }
}


fn density(sample: vec3<f32>, level: f32) -> f32 {
    return textureSampleLevel(DENSITY, SAMPLER, sample, level).x;
}

fn culling(sample: vec3<f32>, level: f32) -> f32 {
    return textureSampleLevel(CULLING, SAMPLER, sample, level).x;
}
