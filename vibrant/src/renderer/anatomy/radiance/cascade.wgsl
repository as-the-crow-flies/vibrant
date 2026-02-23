@group(3) @binding(0) var IRRADIANCE: texture_3d<f32>;
@group(3) @binding(1) var CASCADE_OUT: texture_storage_3d<rgba16float, write>;
@group(3) @binding(2) var CASCADE_IN: texture_3d<f32>;
@group(3) @binding(3) var CASCADE_SAMPLER: sampler;
@group(3) @binding(4) var<uniform> CASCADE_INDEX: u32;

var<private> SOURCE_DIM: vec3<u32>;
var<private> SOURCE_DIM_INV: vec3<f32>;

var<private> CASCADE_OUT_DIM: vec3<u32>;
var<private> CASCADE_IN_DIM: vec3<u32>;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    if (any(texel >= textureDimensions(CASCADE_OUT))) { return; }

    SOURCE_DIM = textureDimensions(ABSORPTION);
    SOURCE_DIM_INV = 1.0 / vec3<f32>(SOURCE_DIM);

    CASCADE_OUT_DIM = textureDimensions(CASCADE_OUT) >> vec3<u32>(CASCADE_INDEX, CASCADE_INDEX, 0u);
    CASCADE_IN_DIM = CASCADE_OUT_DIM >> vec3<u32>(1u);

    let voxel = texel % CASCADE_OUT_DIM;
    let uv = (vec3<f32>(voxel) + 0.5) / vec3<f32>(CASCADE_OUT_DIM);
    let origin = uv * vec3<f32>(SOURCE_DIM);

    if (all(textureSampleLevel(EXTINCTION, SAMPLER, uv, f32(CASCADE_INDEX + 2)) == vec4<f32>(0.0))) { return; }

    // Compute Ray intervals as Harmonic Series (and their cumulative sum)
    // let interval = f32(1 << (2 * CASCADE_INDEX)); // 1 4 16 64 128 256 ...
    // let t0 = (1.0 - interval) / -3.0; // 0 1 5 21 85 213 ...
    // let t1 = t0 + interval; // 1 5 21 85 213 ...

    // Compute Ray intervals as Harmonic Series (and their cumulative sum)
    let interval = f32(1u << (CASCADE_INDEX)); // 1 2 4 8 16 32 ...
    let t0 = interval - 1.0; // 0 1 3 7 15 31 ...
    let t1 = t0 + interval; // 1 3 7 15 31 63 ...

    var radiance = vec3<f32>(0.0, 0.0, 0.0);

    let max_cascade = 5u;

    let direction_count = 2u << CASCADE_INDEX;
    let direction_index = 2u * (texel.xy / CASCADE_OUT_DIM.xy);

    if (ENVIRONMENT.settings.lighting > 0.5) {
        if (CASCADE_INDEX < max_cascade) {
            if (ENVIRONMENT.settings.lighting > 0.75) {
                for (var i=0u; i<2u; i++) {
                    for (var j=0u; j<2u; j++) {
                        let index = direction_index + vec2<u32>(i,j);
                        let da = index_to_direction(index, direction_count);
                        let ri = radiance_interval(origin, da.xyz, t0, t1);

                        radiance += da.a * (ri.radiance + ri.transmission * cascade_radiance(voxel, index));
                    }
                }
            } else {
                let direction = DIRECTION.normal.xyz;
                let ri = radiance_interval(origin, direction, t0, t1);
                radiance = ri.radiance + ri.transmission * cascade_radiance(voxel, vec2<u32>(0));
            }
        } else if (CASCADE_INDEX == max_cascade) {
            radiance = vec3<f32>(1.0);
        }
    } else if (CASCADE_INDEX == 0) {
        radiance = radiance_interval(origin, DIRECTION.normal.xyz, 0.0, 311.0).transmission;
    }

    // radiance = uv;

    textureStore(CASCADE_OUT, texel, vec4<f32>(radiance, 0.0));
}

fn cascade_radiance(voxel: vec3<u32>, direction: vec2<u32>) -> vec3<f32> {
    let origin = CASCADE_IN_DIM * vec3<u32>(direction, 0u);

    if (ENVIRONMENT.settings.radius > 0.5) {
        let texel = origin + (voxel >> vec3<u32>(1u));
        return textureLoad(CASCADE_IN, texel, 0).rgb;
    } else {
        let position = (vec3<f32>(origin) + 0.5 * (vec3<f32>(voxel) + 0.5)) / vec3<f32>(textureDimensions(CASCADE_IN));
        return textureSampleLevel(CASCADE_IN, CASCADE_SAMPLER, position, 0.0).rgb;
    }
}

fn index_to_direction(index: vec2<u32>, size: u32) -> vec4<f32> {
    let delta = 2.0 / f32(size);
    let face = -1.0 + (vec2<f32>(index) + 0.5) * vec2<f32>(delta);
    let area = (delta * delta) / pow(1.0 + dot(face, face), 1.5) * (4.0 * PI);

    let direction = normalize(
        DIRECTION.normal.xyz +
        DIRECTION.tangent.xyz * face.x +
        DIRECTION.bitangent.xyz * face.y
    );

    return vec4<f32>(direction, area);
}

fn radiance_interval(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> RadianceInterval {
    let dim_inv = 1.0 / vec3<f32>(textureDimensions(EXTINCTION));
    let phase_function = 1.0 / (4.0 * PI);
    let step = 1.0;

    var transmission = vec3<f32>(1.0);
    var radiance = vec3<f32>(0.0);

    var outside = 0.0;

    for (var t=t0; t<t1; t+=step) {
        let position_voxel_space = origin + direction * t;
        let sample = position_voxel_space * dim_inv;

        // let irradiance = textureSampleLevel(IRRADIANCE, SAMPLER, sample, 0.0).rgb;
        // let scattering = textureSampleLevel(SCATTERING, SAMPLER, sample, 0.0).rgb;

        // In-Scattering
        // radiance += irradiance * phase_function * scattering * transmission;

        let extinction = textureSampleLevel(EXTINCTION, SAMPLER, sample, 0.0).rgb;

        transmission *= exp(-step * extinction); // TODO: should be in mm
    }

    return RadianceInterval(radiance, transmission);
}
