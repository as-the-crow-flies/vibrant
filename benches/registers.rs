// Per compute shader, what Metal's pipeline reflection says about resources:
// each entry point goes through naga's MSL backend (with wgpu's runtime checks,
// minus naga's thread cap) and Metal's compiler on this GPU.
//
// - max: threads a threadgroup may hold. It only drops below 1024 under heavy
//   register pressure, so on Apple GPUs it flags extreme cases, not occupancy.
// - shared bytes: workgroup memory per threadgroup (from naga's layout; Metal
//   doesn't count memory bound at dispatch), which bounds how many threadgroups
//   fit on a core.
//
//   cargo bench --bench registers            # every compute shader
//   cargo bench --bench registers -- cascade # paths containing "cascade"
//
// Render pipelines (the volume and line tracers) report no such limits. Real
// occupancy and limiter counters: `cargo bench --bench counters`.

#[cfg(target_vendor = "apple")]
fn main() {
    use naga::back::msl;
    use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
    use naga::valid::{
        Capabilities, ShaderStages, SubgroupOperationSet, ValidationFlags, Validator,
    };
    use objc2_foundation::NSString;
    use objc2_metal::{MTLCompileOptions, MTLComputePipelineState, MTLDevice, MTLLibrary};

    let filter = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .unwrap_or_default();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("no Metal adapter");
    let (device, _queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("no device");
    let hal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.expect("not Metal");
    let metal = hal.raw_device();

    let checked = BoundsCheckPolicy::Restrict;
    let options = msl::Options {
        lang_version: (3, 2),
        fake_missing_bindings: true,
        bounds_check_policies: BoundsCheckPolicies {
            index: checked,
            buffer: checked,
            image_load: checked,
            binding_array: BoundsCheckPolicy::Unchecked,
        },
        zero_initialize_workgroup_memory: true,
        force_loop_bounding: true,
        emit_int_div_checks: true,
        ..Default::default()
    };

    println!(
        "{:<52} {:>10} {:>9} {:>6} {:>12}",
        "shader / entry point", "workgroup", "max", "simd", "shared bytes"
    );
    for (path, source) in vibrant::renderer::wgsl::roots() {
        if !path.contains(filter.as_str()) || !source.contains("@compute") {
            continue;
        }

        let module =
            naga::front::wgsl::parse_str(source).unwrap_or_else(|error| panic!("{path}: {error}"));
        let info = Validator::new(
            ValidationFlags::all(),
            Capabilities::SUBGROUP | Capabilities::SHADER_FLOAT16_IN_FLOAT32,
        )
        .subgroup_stages(ShaderStages::all())
        .subgroup_operations(SubgroupOperationSet::all())
        .validate(&module)
        .unwrap_or_else(|error| panic!("{path}: {error}"));

        let mut layouter = naga::proc::Layouter::default();
        layouter.update(module.to_ctx()).unwrap();

        for entry in module
            .entry_points
            .iter()
            .filter(|entry| entry.stage == naga::ShaderStage::Compute)
        {
            let pipeline = msl::PipelineOptions {
                entry_point: Some((naga::ShaderStage::Compute, entry.name.clone())),
                ..Default::default()
            };
            let (code, translation) = msl::write_string(&module, &info, &options, &pipeline)
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            let function = translation.entry_point_names[0]
                .as_ref()
                .expect("entry point name");
            // naga caps the pipeline at the workgroup size; without that the
            // limit is the registers'.
            let code = uncap(&code);

            let library = metal
                .newLibraryWithSource_options_error(
                    &NSString::from_str(&code),
                    Some(&MTLCompileOptions::new()),
                )
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            let kernel = library
                .newFunctionWithName(&NSString::from_str(function))
                .expect("function");
            let state = metal
                .newComputePipelineStateWithFunction_error(&kernel)
                .unwrap_or_else(|error| panic!("{path}: {error}"));

            let size = entry.workgroup_size;
            let workgroup = format!("{}x{}x{}", size[0], size[1], size[2]);
            let threads = (size[0] * size[1] * size[2]) as usize;
            let max = state.maxTotalThreadsPerThreadgroup();
            // Workgroup memory is bound at dispatch, so Metal doesn't count it.
            let shared: u32 = module
                .global_variables
                .iter()
                .filter(|(handle, var)| {
                    var.space == naga::AddressSpace::WorkGroup
                        && !info.get_entry_point(entry_index(&module, entry))[*handle].is_empty()
                })
                .map(|(_, var)| layouter[var.ty].size)
                .sum();
            let flag = if max < threads {
                "  WON'T RUN"
            } else if max < 1024 {
                "  register-bound"
            } else {
                ""
            };
            println!(
                "{:<52} {:>10} {:>9} {:>6} {:>12}{flag}",
                format!("{path} {}", entry.name),
                workgroup,
                max,
                state.threadExecutionWidth(),
                shared,
            );
        }
    }
}

#[cfg(target_vendor = "apple")]
fn uncap(code: &str) -> String {
    const ATTRIBUTE: &str = "[[max_total_threads_per_threadgroup(";
    match code.find(ATTRIBUTE) {
        Some(start) => {
            let end = start + code[start..].find("]] ").unwrap() + 3;
            format!("{}{}", &code[..start], &code[end..])
        }
        None => code.to_string(),
    }
}

#[cfg(target_vendor = "apple")]
fn entry_index(module: &naga::Module, entry: &naga::EntryPoint) -> usize {
    module
        .entry_points
        .iter()
        .position(|e| std::ptr::eq(e, entry))
        .unwrap()
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    eprintln!("registers: Metal only");
}
