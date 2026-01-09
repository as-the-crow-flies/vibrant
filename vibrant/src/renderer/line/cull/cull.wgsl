@group(0) @binding(0) var CULLING: texture_storage_3d<r32float, read_write>;
@group(1) @binding(0) var DENSITY: texture_storage_3d<r32float, read_write>;
@group(2) @binding(3) var DENSITY_ERODED: texture_storage_3d<r32float, read_write>;
@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) this_voxel: vec3<u32>) {
    let dim = textureDimensions(CULLING).x;

    let dim_f32 = f32(dim);
    let one_over_dim = 1.0 / dim_f32;

    let this_voxel_density = saturate(textureLoad(DENSITY, this_voxel).x);

    let max_density = 4.0;

    var keep = ENVIRONMENT.settings.culling == 0 && this_voxel_density > 0.0;

    if (!keep && this_voxel_density > 0.0) {
        let position = vec3<f32>(this_voxel) + 0.5;
        let camera = dim_f32 * (ENVIRONMENT.camera.transform[3].xyz + 0.5);

        let delta = camera - position;

        let distance = length(delta);
        let direction = delta / distance;
        let voxel_boundaries = 1.0 / abs(direction);
        let step = vec3<i32>(sign(direction));
        var next = vec4<f32>(
            one_if_zero(abs(fract(vec3<f32>(-step) * fract(position)))) * voxel_boundaries,
            min(distance, 16.0)
        );

        var voxel = vec3<i32>(position);

        var total_density = 0.0;

        while (next.w > 0.0 && total_density <= max_density) {
            let increment = minimum(next);
            let mask = next == vec4<f32>(increment);

            voxel += step * vec3<i32>(mask.xyz);
            next = select(next - increment, vec4<f32>(voxel_boundaries, 0.0), mask);

            total_density += increment * textureLoad(DENSITY_ERODED, voxel).x;
        }

        keep = total_density <= max_density;
    }

    textureStore(CULLING, this_voxel, vec4<f32>(f32(keep)));
}

fn one_if_zero(v: vec3<f32>) -> vec3<f32> {
    return v + vec3<f32>(v == vec3<f32>(0.0));
}

fn minimum(v: vec4<f32>) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}
