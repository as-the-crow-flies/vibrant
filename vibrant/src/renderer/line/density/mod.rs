use std::any::type_name;

use wgpu::{CommandEncoder, ComputePassDescriptor, ComputePipeline, PipelineLayoutDescriptor};

use crate::{
    asset::{
        line::LineSet,
        scalar::{R16Uint, R32Float, Rgba8Unorm, ScalarTexture3D},
    },
    controller::settings::VoxelizationSetting,
    gpu::Gpu,
    renderer::{environment::Environment, wgsl},
    surface::{density::Density, Frame},
};

pub struct DensityPipeline {
    voxelize_tube: ComputePipeline,
    voxelize_line: ComputePipeline,
    voxelize_box: ComputePipeline,
    copy: ComputePipeline,
    color: ComputePipeline,
    mipmap: ComputePipeline,
}

impl DensityPipeline {
    pub fn new(gpu: &Gpu) -> Self {
        let label = Some(type_name::<Self>());

        let voxelize_layout = &gpu
            .device()
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[
                    &Density::layout(gpu),
                    &LineSet::layout(gpu, true),
                    &Environment::layout(gpu),
                ],
                push_constant_ranges: &[],
            });

        let voxelize_shader_source = include_str!("voxelize.wgsl");

        Self {
            voxelize_tube: gpu.compute(
                "Density::Voxelize::Tube",
                voxelize_layout,
                &gpu.shader(&(wgsl::voxelize::TUBE.to_owned() + voxelize_shader_source)),
            ),
            voxelize_line: gpu.compute(
                "Density::Voxelize::Line",
                voxelize_layout,
                &gpu.shader(&(wgsl::voxelize::LINE.to_owned() + voxelize_shader_source)),
            ),
            voxelize_box: gpu.compute(
                "Density::Voxelize::Box",
                voxelize_layout,
                &gpu.shader(&(wgsl::voxelize::BOX.to_owned() + voxelize_shader_source)),
            ),
            copy: gpu.compute(
                "Density::Copy",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &Density::layout(gpu),
                            &ScalarTexture3D::<R32Float>::layout_write(gpu),
                            &ScalarTexture3D::<R16Uint>::layout_write(gpu),
                            &Environment::layout(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("copy.wgsl")),
            ),
            color: gpu.compute(
                "Density::Color",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[
                            &ScalarTexture3D::<R32Float>::layout(gpu),
                            &ScalarTexture3D::<Rgba8Unorm>::layout_write(gpu),
                            &Environment::layout(gpu),
                        ],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("color.wgsl")),
            ),
            mipmap: gpu.compute(
                "Density::MipMap",
                &gpu.device()
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label,
                        bind_group_layouts: &[&ScalarTexture3D::<R32Float>::layout_mipmap(gpu)],
                        push_constant_ranges: &[],
                    }),
                &gpu.shader(include_str!("mipmap.wgsl")),
            ),
        }
    }

    pub fn render(
        &self,
        cmd: &mut CommandEncoder,
        frame: &Frame,
        environment: &Environment,
        setting: VoxelizationSetting,
        tractogram: &LineSet,
    ) {
        frame.density().clear(cmd);
        tractogram.clear_count(cmd);

        let mut pass = cmd.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Density"),
            ..Default::default()
        });

        let n = frame.density().density().size().div_ceil(8);

        pass.set_bind_group(0, frame.density().binding(), &[]);
        pass.set_bind_group(1, tractogram.binding(true), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);

        pass.set_pipeline(match setting {
            VoxelizationSetting::Line => &self.voxelize_line,
            VoxelizationSetting::Box => &self.voxelize_box,
            VoxelizationSetting::Tube => &self.voxelize_tube,
        });
        pass.dispatch_workgroups(64, 1, 1);

        pass.set_pipeline(&self.copy);
        pass.set_bind_group(0, frame.density().binding(), &[]);
        pass.set_bind_group(1, frame.density().density().binding_write(), &[]);
        pass.set_bind_group(2, frame.density().count().binding_write(), &[]);
        pass.set_bind_group(3, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.color);
        pass.set_bind_group(0, frame.density().density().binding(), &[]);
        pass.set_bind_group(1, frame.density().color().binding_write(), &[]);
        pass.set_bind_group(2, environment.binding(), &[]);
        pass.dispatch_workgroups(n, n, n);

        pass.set_pipeline(&self.mipmap);

        let mut mipmap = frame.density().density().size().div_ceil(8);

        for binding in frame.density().density().bindings_mipmap() {
            pass.set_bind_group(0, binding, &[]);
            pass.dispatch_workgroups(mipmap, mipmap, mipmap);

            mipmap = mipmap.div_ceil(2);
        }
    }
}
