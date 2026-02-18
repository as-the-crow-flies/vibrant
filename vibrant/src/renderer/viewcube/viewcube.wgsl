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
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.face_id = input.face_id;

    // Background quad (face_id=255): pass through NDC position directly, no rotation
    if input.face_id == 255u {
        output.position = vec4<f32>(input.position, 1.0);
        output.normal = vec3<f32>(0.0, 0.0, 1.0);
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
    let z_ndc = rotated.z * 0.5 + 0.5; // Map z to [0,1] for depth

    output.position = vec4<f32>(x_ndc, y_ndc, z_ndc, 1.0);
    output.normal = rotated_normal;
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

    // Visual mode: flat-shaded lighting
    let base = face_color(input.face_id);

    // Simple directional light from upper-right-front
    let light_dir = normalize(vec3<f32>(0.4, 0.7, 0.5));
    let n = normalize(input.normal);
    let ndotl = max(dot(n, light_dir), 0.0);

    // Ambient + diffuse
    let ambient = 0.35;
    let diffuse = 0.65;
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

    return vec4<f32>(color, 1.0);
}
