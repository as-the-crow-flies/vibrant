// View Cube Shader
// Renders a flat-shaded cube with per-region colors (faces, edges, corners).
// 26 pick targets: 6 faces (0-5), 12 edges (6-17), 8 corners (18-25).
// Two modes: visual (pick_mode=0) and pick (pick_mode=1).

struct Uniforms {
    rotation: mat4x4<f32>,     // Camera rotation (inverse) applied to cube
    viewport: vec4<f32>,       // x, y, width, height in pixels (NDC computation)
    screen_size: vec2<f32>,    // Full screen width, height
    pick_mode: u32,            // 0 = visual, 1 = pick
    hovered_id: u32,           // ID of hovered element (255 = none)
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var label_tex: texture_2d<f32>;
@group(0) @binding(2) var label_samp: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) face_id: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) @interpolate(flat) face_id: u32,
    @location(2) world_pos: vec3<f32>,
    @location(3) object_normal: vec3<f32>,
    @location(4) object_pos: vec3<f32>,
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.face_id = input.face_id;

    output.object_pos = input.position;

    // Background quad (face_id=255): pass through NDC position directly, no rotation
    if input.face_id == 255u {
        output.position = vec4<f32>(input.position, 1.0);
        output.normal = vec3<f32>(0.0, 0.0, 1.0);
        output.object_normal = vec3<f32>(0.0, 0.0, 1.0);
        output.world_pos = input.position;
        return output;
    }

    // Rotate the cube by the inverse camera rotation so it reflects the scene orientation
    let rotated = u.rotation * vec4<f32>(input.position, 1.0);
    let rotated_normal = (u.rotation * vec4<f32>(input.normal, 0.0)).xyz;

    // Simple orthographic projection filling the viewport.
    // The cube is in [-1,1], scale down to ~60% of the viewport.
    let scale = 0.6;
    let x_ndc = rotated.x * scale;
    let y_ndc = rotated.y * scale;
    // Map z to [0,1] for depth. The cube diagonal extends to sqrt(3) ≈ 1.73,
    // so divide by 2*sqrt(3) ≈ 3.4641 to ensure all vertices stay within [0,1] clip range.
    let z_ndc = rotated.z / (2.0 * sqrt(3.0)) + 0.5;

    output.position = vec4<f32>(x_ndc, y_ndc, z_ndc, 1.0);
    output.normal = rotated_normal;
    output.object_normal = input.normal;
    output.world_pos = rotated.xyz;

    return output;
}

// Base face colors (muted, professional palette)
fn base_face_color(id: u32) -> vec3<f32> {
    switch id {
        case 0u: { return vec3<f32>(0.55, 0.63, 0.75); } // Front  - steel blue
        case 1u: { return vec3<f32>(0.55, 0.63, 0.75); } // Back   - steel blue
        case 2u: { return vec3<f32>(0.65, 0.55, 0.55); } // Right  - dusty rose
        case 3u: { return vec3<f32>(0.65, 0.55, 0.55); } // Left   - dusty rose
        case 4u: { return vec3<f32>(0.55, 0.68, 0.55); } // Top    - sage green
        case 5u: { return vec3<f32>(0.55, 0.68, 0.55); } // Bottom - sage green
        default: { return vec3<f32>(0.6, 0.6, 0.6); }
    }
}

// Get visual color for any pick ID (0-25).
// Faces: base color. Edges: average of 2 adjacent faces. Corners: average of 3.
fn face_color(id: u32) -> vec3<f32> {
    switch id {
        // 6 Face centers
        case 0u: { return base_face_color(0u); }
        case 1u: { return base_face_color(1u); }
        case 2u: { return base_face_color(2u); }
        case 3u: { return base_face_color(3u); }
        case 4u: { return base_face_color(4u); }
        case 5u: { return base_face_color(5u); }
        // 12 Edges (blend of 2 adjacent face colors)
        case  6u: { return mix(base_face_color(0u), base_face_color(2u), 0.5); } // Front-Right
        case  7u: { return mix(base_face_color(0u), base_face_color(3u), 0.5); } // Front-Left
        case  8u: { return mix(base_face_color(0u), base_face_color(4u), 0.5); } // Front-Top
        case  9u: { return mix(base_face_color(0u), base_face_color(5u), 0.5); } // Front-Bottom
        case 10u: { return mix(base_face_color(1u), base_face_color(2u), 0.5); } // Back-Right
        case 11u: { return mix(base_face_color(1u), base_face_color(3u), 0.5); } // Back-Left
        case 12u: { return mix(base_face_color(1u), base_face_color(4u), 0.5); } // Back-Top
        case 13u: { return mix(base_face_color(1u), base_face_color(5u), 0.5); } // Back-Bottom
        case 14u: { return mix(base_face_color(2u), base_face_color(4u), 0.5); } // Right-Top
        case 15u: { return mix(base_face_color(2u), base_face_color(5u), 0.5); } // Right-Bottom
        case 16u: { return mix(base_face_color(3u), base_face_color(4u), 0.5); } // Left-Top
        case 17u: { return mix(base_face_color(3u), base_face_color(5u), 0.5); } // Left-Bottom
        // 8 Corners (blend of 3 adjacent face colors)
        case 18u: { return (base_face_color(0u) + base_face_color(2u) + base_face_color(4u)) / 3.0; } // Front-Right-Top
        case 19u: { return (base_face_color(0u) + base_face_color(3u) + base_face_color(4u)) / 3.0; } // Front-Left-Top
        case 20u: { return (base_face_color(0u) + base_face_color(2u) + base_face_color(5u)) / 3.0; } // Front-Right-Bottom
        case 21u: { return (base_face_color(0u) + base_face_color(3u) + base_face_color(5u)) / 3.0; } // Front-Left-Bottom
        case 22u: { return (base_face_color(1u) + base_face_color(2u) + base_face_color(4u)) / 3.0; } // Back-Right-Top
        case 23u: { return (base_face_color(1u) + base_face_color(3u) + base_face_color(4u)) / 3.0; } // Back-Left-Top
        case 24u: { return (base_face_color(1u) + base_face_color(2u) + base_face_color(5u)) / 3.0; } // Back-Right-Bottom
        case 25u: { return (base_face_color(1u) + base_face_color(3u) + base_face_color(5u)) / 3.0; } // Back-Left-Bottom
        default: { return vec3<f32>(0.6, 0.6, 0.6); }
    }
}

// Compute face-local UV from object-space position.
// The face center cell spans [-0.5, 0.5] in face-local axes (edge_t = 0.25, cube half-extent = 1).
// We remap that range to [0,1] so the entire label cell fills the center region.
// face_id 0-5: Front(+Z), Back(-Z), Right(+X), Left(-X), Top(+Y), Bottom(-Y)
fn face_uv(face_id: u32, pos: vec3<f32>) -> vec2<f32> {
    // Remap: val in [-0.5, 0.5] -> [0, 1]
    // remap(x) = x + 0.5
    switch face_id {
        case 0u: { // Front (+Z): use x,y
            return vec2<f32>(pos.x + 0.5, 1.0 - (pos.y + 0.5));
        }
        case 1u: { // Back (-Z): use -x,y (mirrored)
            return vec2<f32>(-pos.x + 0.5, 1.0 - (pos.y + 0.5));
        }
        case 2u: { // Right (+X): use -z,y
            return vec2<f32>(-pos.z + 0.5, 1.0 - (pos.y + 0.5));
        }
        case 3u: { // Left (-X): use z,y
            return vec2<f32>(pos.z + 0.5, 1.0 - (pos.y + 0.5));
        }
        case 4u: { // Top (+Y): use x,-z
            return vec2<f32>(pos.x + 0.5, pos.z + 0.5);
        }
        case 5u: { // Bottom (-Y): use x,z
            return vec2<f32>(pos.x + 0.5, 1.0 - (pos.z + 0.5));
        }
        default: {
            return vec2<f32>(0.5, 0.5);
        }
    }
}

// Sample the label atlas. The atlas is 6 rows stacked vertically (one per face).
// face_id selects which row, uv selects the position within that row.
fn sample_label(face_id: u32, uv: vec2<f32>) -> f32 {
    // Each row occupies 1/6 of the atlas vertically
    let row = f32(face_id);
    let atlas_uv = vec2<f32>(uv.x, (row + uv.y) / 6.0);
    let sample = textureSample(label_tex, label_samp, atlas_uv);
    return sample.a;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Pick mode: encode face_id in red channel (skip background quad)
    if u.pick_mode == 1u {
        if input.face_id == 255u {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0); // transparent background for pick
        }
        let encoded = f32(input.face_id + 1u) / 255.0;
        return vec4<f32>(encoded, 0.0, 0.0, 1.0);
    }

    // Background quad: dark semi-transparent backdrop
    if input.face_id == 255u {
        return vec4<f32>(0.12, 0.12, 0.14, 0.85);
    }

    // Visual mode: flat-shaded lighting in object space.
    // Using the unrotated (object-space) normal ensures lighting stays
    // consistent regardless of camera orientation — no face goes fully dark.
    let base = face_color(input.face_id);

    // Light direction in object space (upper-right-front of the widget, not the scene)
    let light_dir = normalize(vec3<f32>(0.4, 0.7, 0.5));
    let n = normalize(input.object_normal);
    let ndotl = max(dot(n, light_dir), 0.0);

    // Ambient + diffuse
    let ambient = 0.45;
    let diffuse = 0.55;
    var color = base * (ambient + diffuse * ndotl);

    // Edge darkening: darken edge/corner regions slightly
    if input.face_id >= 6u && input.face_id <= 17u {
        // Edges: slight darkening
        color = color * 0.88;
    } else if input.face_id >= 18u {
        // Corners: more darkening
        color = color * 0.78;
    }

    // Hover highlight
    if input.face_id == u.hovered_id {
        color = mix(color, vec3<f32>(1.0, 1.0, 1.0), 0.3);
    }

    // Label text: only for face center IDs (0-5)
    if input.face_id <= 5u {
        let raw_uv = face_uv(input.face_id, input.object_pos);
        let uv = clamp(raw_uv, vec2<f32>(0.0), vec2<f32>(1.0));
        let text_alpha = sample_label(input.face_id, uv);
        // Composite white text onto face color
        let text_color = vec3<f32>(1.0, 1.0, 1.0);
        color = mix(color, text_color, text_alpha * 0.95);
    }

    return vec4<f32>(color, 1.0);
}
