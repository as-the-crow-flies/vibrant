// Transfer function of one fraction. The importer binds `MATERIAL` and `COLORMAP`.

struct MaterialNode {
    absorption: vec3<f32>,
    position: f32,
    scattering: vec3<f32>,
    ior: f32,
}

struct MaterialSample {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    ior: f32,
}

struct Material {
    inverted: u32,
    masked: u32,
    use_colormap: u32,
    colormap: u32,
    // Scales absorption + scattering after the transfer function. `_pad` keeps
    // `nodes` on a 16-byte boundary (see VolumeFractionSettingsBuffer).
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    nodes: array<MaterialNode, 8>,
}

// Material at transfer-function position `fraction`, retinted by the colormap
// and scaled by opacity.
fn classify(fraction: f32) -> MaterialSample {
    var material = sample_material(fraction);

    // Same asymmetric hue trick as before (absorption takes the inverted hue so
    // it doesn't cancel out the scattered color). Intensity is already baked
    // into `material`, so this only retints it.
    if (bool(MATERIAL.use_colormap) && fraction != 0.0) {
        let tint = textureLoad(COLORMAP, vec3<u32>(u32(fraction * 255.0), MATERIAL.colormap, 0u), 0).rgb;
        material.absorption *= invert_hue_approx(tint);
        material.scattering *= tint;
    }

    // ior is density-weighted downstream, so its average is unchanged.
    let opacity = saturate(MATERIAL.opacity);
    material.absorption *= opacity;
    material.scattering *= opacity;

    return material;
}

// Piecewise-linear lookup across MATERIAL.nodes at transfer-function position `t`,
// mirroring TransferFunctionEditor::gradient() on the Rust side.
fn sample_material(t: f32) -> MaterialSample {
    var count = 0u;

    for (var i = 0u; i < 8u; i = i + 1u) {
        if (MATERIAL.nodes[i].ior <= 0.0) { break; }
        count = count + 1u;
    }

    if (count == 0u) {
        return MaterialSample(vec3<f32>(0.0), vec3<f32>(0.0), 0.0);
    }

    if (count == 1u) {
        return MaterialSample(MATERIAL.nodes[0].absorption, MATERIAL.nodes[0].scattering, MATERIAL.nodes[0].ior);
    }

    var index = count;

    for (var i = 0u; i < count; i = i + 1u) {
        if (MATERIAL.nodes[i].position >= t) {
            index = i;
            break;
        }
    }

    if (index == 0u) {
        return MaterialSample(MATERIAL.nodes[0].absorption, MATERIAL.nodes[0].scattering, MATERIAL.nodes[0].ior);
    }

    if (index == count) {
        let node = MATERIAL.nodes[count - 1u];
        return MaterialSample(node.absorption, node.scattering, node.ior);
    }

    let a = MATERIAL.nodes[index - 1u];
    let b = MATERIAL.nodes[index];
    let span = max(b.position - a.position, 0.00001);
    let factor = saturate((t - a.position) / span);

    return MaterialSample(
        mix(a.absorption, b.absorption, factor),
        mix(a.scattering, b.scattering, factor),
        mix(a.ior, b.ior, factor)
    );
}

fn invert_hue_approx(color: vec3<f32>) -> vec3<f32> {
    let luma = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return vec3<f32>(2.0 * luma) - color;
}
