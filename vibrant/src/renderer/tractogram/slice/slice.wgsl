@group(0) @binding(0) var DENSITY: texture_3d<f32>;
@group(0) @binding(1) var DENSITY_SAMPLER: sampler;

@group(1) @binding(0) var HIZ: texture_storage_2d<r8unorm, read_write>;

@group(2) @binding(1) var BINS: texture_storage_3d<r8uint, read_write>;
@group(2) @binding(2) var LOZ: texture_storage_2d<r8unorm, read_write>;
@group(2) @binding(3) var ABSORBANCE: texture_storage_2d<r8unorm, read_write>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dim = vec3<f32>(textureDimensions(DENSITY));

    let tile = id.xy;

    // Get near and far plane in 3D texture space
    let uv = 2.0 * vec2<f32>(tile * ENVIRONMENT.tile) / vec2<f32>(ENVIRONMENT.surface) - 1.0;
    let near = unproject(vec4<f32>(uv.xy, 0.0, 1.0)) + 0.5;
    let far = unproject(vec4<f32>(uv.xy, 1.0, 1.0)) + 0.5;

    let delta = far - near;
    let distance = length(delta);

    var max_absorbance = 0.0;
    var loz = 0.0;
    var hiz = 1.0;
    {
        let direction = delta / distance;
        let step = 1.0 / maximum(abs(direction * dim));
        let factor = distance * step * dim.x;

        for (var depth = 0.0; depth < 1.0; depth += step) {
            max_absorbance += factor * density(mix(near, far, depth));

            if (max_absorbance == 0.0) {
                loz = depth;
            }

            if (max_absorbance > ENVIRONMENT.settings.culling_threshold) {
                max_absorbance = ENVIRONMENT.settings.culling_threshold;
                hiz = depth;

                break;
            }
        }
    }

    textureStore(LOZ, tile, vec4<f32>(loz));
    textureStore(HIZ, tile, vec4<f32>(hiz));
    textureStore(ABSORBANCE, tile, vec4<f32>(precision_encode(max_absorbance / ENVIRONMENT.settings.culling_threshold)));

    let absorbance_per_layer = max_absorbance / f32(ENVIRONMENT.layers);

    var layer = 0u;
    var absorbance = 0.0;
    {
        let step = U8_MAX_INV * (hiz - loz);
        let factor = distance * step * dim.x;

        for (var index = 0u; index <= U8_MAX; index++) {
            let depth = loz + f32(index) * step;

            absorbance += factor * density(mix(near, far, depth));

            textureStore(BINS, vec3<u32>(tile, index), vec4<u32>(layer));

            if (absorbance > f32(layer) * absorbance_per_layer) {
                layer = min(layer + 1, ENVIRONMENT.layers - 1);
            }
        }
    }
}

fn unproject(v: vec4<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * v;
    return t.xyz / t.w;
}

fn maximum(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn density(sample: vec3<f32>) -> f32 {
    return precision_decode(textureSampleLevel(DENSITY, DENSITY_SAMPLER, sample, 0.0).x);
}
