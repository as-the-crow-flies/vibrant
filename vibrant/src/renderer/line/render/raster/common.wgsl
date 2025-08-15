const K: u32 = 8u;

// Adapted from David Groß and Stefan Gumhold 2020
fn generate_view_aligned_quad(eye: vec3<f32>, view: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> mat4x4<f32> {
	let delta = normalize(pb - pa);

	var a_to_eye = eye - pa;
	var b_to_eye = eye - pb;

	let eye_dists = vec2<f32>(length(a_to_eye), length(b_to_eye));

	let pers_rad_scales = r / sqrt(eye_dists*eye_dists - r*r);

	let pers_rads = eye_dists * pers_rad_scales;
	var pers_max_rads = eye_dists * vec2<f32>(max(pers_rad_scales.x, pers_rad_scales.y));

	a_to_eye /= eye_dists.x;
	b_to_eye /= eye_dists.y;

	let aligned_up = normalize(cross(delta, a_to_eye));
	let bitangent0 = cross(aligned_up, a_to_eye);
	let bitangent1 = cross(aligned_up, b_to_eye);

	let p0 = pa + pers_rads.x * bitangent0;
	let p1 = pa - pers_rads.x * bitangent0;
	let p2 = pb + pers_rads.y * bitangent1;
	let p3 = pb - pers_rads.y * bitangent1;

	let test_dir = normalize(cross(aligned_up, view));

	let ang0 = dot(normalize(p0 - eye), test_dir);
	let ang1 = dot(normalize(p1 - eye), test_dir);
	let ang2 = dot(normalize(p2 - eye), test_dir);
	let ang3 = dot(normalize(p3 - eye), test_dir);

	var a = p0;
	var b = p3;

	if(ang0 > ang2) {
		a = p2;
		pers_max_rads.x = pers_max_rads.y;
	}

	if(ang1 > ang3) {
		b = p1;
		pers_max_rads.y = pers_max_rads.x;
	}

	return mat4x4<f32>(
    	vec4<f32>(a - pers_max_rads.x * aligned_up, 1.0),
    	vec4<f32>(a + pers_max_rads.x * aligned_up, 1.0),
    	vec4<f32>(b - pers_max_rads.y * aligned_up, 1.0),
    	vec4<f32>(b + pers_max_rads.y * aligned_up, 1.0)
	);
}

// Adapted from David Groß and Stefan Gumhold 2020
fn generate_aligned_box_billboard(eye: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, rm: f32) -> mat4x4<f32> {
	let ra = rm;
	let rb = rm;

    let center = 0.5 * (pa + pb);

	let delta = normalize(pb - pa);
	let local_z = normalize(center - eye);
	let local_y = select(
    	ortho_vec(local_z),
    	normalize(cross(delta, local_z)),
        abs(dot(delta, local_z)) < 0.99999
	);

	let local_x = cross(local_z, local_y);

	// translate positions by local coordinate system origin
	let pac = pa - center;
	let pbc = pb - center;

	// project xz coords of positions onto local coordinate system axes (rotate in local coordinate system)
	let pl0x = dot(pac, local_x);
	let pl0z = dot(pac, local_z);

	let pl1x = dot(pbc, local_x);
	let pl1z = dot(pbc, local_z);

	let xl = max(pl1x - pl0x, 0.0);
	let xm = min(-ra, xl - rb);
	let xp = max( ra, xl + rb);

	let zl = pl1z - pl0z;
	let zm = min(-ra, zl - rb);

	let dy = rm + rm;

	let p00 = vec2<f32>(pl0x + xm, -rm);
	let p10 = vec2<f32>(pl0x + xp, -rm);
	let p01 = vec2<f32>(p00.x, rm);
	let p11 = vec2<f32>(p10.x, rm);

	let local_z_scaled = (pl0z + zm) * local_z;

	var ret = mat4x4<f32>();
	ret[0] = vec4(center + p00.x * local_x + p00.y * local_y + local_z_scaled, 1.0);
	ret[1] = vec4(center + p10.x * local_x + p10.y * local_y + local_z_scaled, 1.0);
	ret[2] = vec4(center + p01.x * local_x + p01.y * local_y + local_z_scaled, 1.0);
	ret[3] = ret[1] + ret[2] - ret[0];
	return ret;
}

fn ortho_vec(v: vec3<f32>) -> vec3<f32> {
    return select(vec3<f32>(0.0, -v.z, v.y), vec3<f32>(-v.y, v.x, 0.0), abs(v.x) > abs(v.z));
}
