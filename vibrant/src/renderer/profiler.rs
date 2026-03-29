// GPU timestamp query profiler — measures GPU time per render pass

use bytemuck::cast_slice;
use futures::channel::oneshot::channel;
use pollster::FutureExt;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Features, MapMode, QuerySet,
    QuerySetDescriptor, QueryType,
};

use crate::gpu::Gpu;

// Pass index constants — order must match PASS_LABELS below.
pub const PASS_TRANSFORM: usize = 0;
pub const PASS_CROP: usize = 1;
pub const PASS_OCCUPANCY: usize = 2;
pub const PASS_CULL: usize = 3;
pub const PASS_OCCLUSION: usize = 4;
pub const PASS_POPULATE: usize = 5;
pub const PASS_RENDER: usize = 6;
pub const PASS_POST: usize = 7;
pub const PASS_AA: usize = 8;

pub const PASS_LABELS: &[&str] = &[
    "transform",
    "crop",
    "occupancy",
    "cull",
    "occlusion",
    "populate",
    "render",
    "post",
    "aa",
];

// GpuProfiler wraps timestamp query state and readback buffers.
pub struct GpuProfiler {
    enabled: bool,
    query_set: Option<QuerySet>,
    // GPU-side resolved timestamps (QUERY_RESOLVE | COPY_SRC)
    resolve_buffer: Option<Buffer>,
    // CPU-readable copy of resolved timestamps (COPY_DST | MAP_READ)
    readback_buffer: Option<Buffer>,
    // nanoseconds per timestamp tick (from queue.get_timestamp_period())
    timestamp_period: f32,
    // Last-frame results in milliseconds, indexed by PASS_* constants
    pub results: Vec<f32>,
}

impl GpuProfiler {
    pub fn new(gpu: &Gpu) -> Self {
        // TIMESTAMP_QUERY for resolve_query_set
        // TIMESTAMP_QUERY_INSIDE_ENCODERS for write_timestamp on CommandEncoder
        let ts_features = Features::TIMESTAMP_QUERY | Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let enabled = gpu.device().features().contains(ts_features)
            && gpu.queue().get_timestamp_period() > 0.0;

        if !enabled {
            return Self {
                enabled: false,
                query_set: None,
                resolve_buffer: None,
                readback_buffer: None,
                timestamp_period: 1.0,
                results: vec![0.0; PASS_LABELS.len()],
            };
        }

        let count = PASS_LABELS.len() as u32 * 2; // start + end per pass
        let size = count as u64 * 8; // u64 per timestamp = 8 bytes

        let query_set = gpu.device().create_query_set(&QuerySetDescriptor {
            label: Some("GpuProfiler::QuerySet"),
            ty: QueryType::Timestamp,
            count,
        });

        let resolve_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("GpuProfiler::Resolve"),
            size,
            usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = gpu.device().create_buffer(&BufferDescriptor {
            label: Some("GpuProfiler::Readback"),
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            enabled: true,
            query_set: Some(query_set),
            resolve_buffer: Some(resolve_buffer),
            readback_buffer: Some(readback_buffer),
            timestamp_period: gpu.queue().get_timestamp_period(),
            results: vec![0.0; PASS_LABELS.len()],
        }
    }

    // write start timestamp for pass `index`
    pub fn begin(&self, cmd: &mut CommandEncoder, index: usize) {
        if let Some(qs) = &self.query_set {
            cmd.write_timestamp(qs, (index * 2) as u32);
        }
    }

    // write end timestamp for pass `index`
    pub fn end(&self, cmd: &mut CommandEncoder, index: usize) {
        if let Some(qs) = &self.query_set {
            cmd.write_timestamp(qs, (index * 2 + 1) as u32);
        }
    }

    // resolve query set → resolve_buffer → readback_buffer
    // must be called at the end of the command encoder, before submission
    pub fn resolve(&self, cmd: &mut CommandEncoder) {
        let (Some(qs), Some(resolve), Some(readback)) =
            (&self.query_set, &self.resolve_buffer, &self.readback_buffer)
        else {
            return;
        };
        let count = PASS_LABELS.len() as u32 * 2;
        cmd.resolve_query_set(qs, 0..count, resolve, 0);
        cmd.copy_buffer_to_buffer(resolve, 0, readback, 0, readback.size());
    }

    // read back timestamp results after the frame has been submitted and waited on
    // updates `self.results` with per-pass GPU time in milliseconds
    pub fn collect(&mut self, gpu: &Gpu) {
        if let Some(new_results) = self.read_timestamps(gpu) {
            self.results = new_results;
        }
    }

    // maps readback_buffer synchronously, reads u64 timestamps, converts to ms
    fn read_timestamps(&self, gpu: &Gpu) -> Option<Vec<f32>> {
        let readback = self.readback_buffer.as_ref()?;
        let period = self.timestamp_period;

        let (sender, receiver) = channel();
        readback.slice(..).map_async(MapMode::Read, move |r| {
            let _ = sender.send(r);
        });
        // process the map callback (GPU work already done by the time we get here)
        gpu.wait();

        let map_ok = receiver.block_on().ok()?.is_ok();
        if !map_ok {
            return None;
        }

        let results = {
            let view = readback.slice(..).get_mapped_range();
            let timestamps: &[u64] = cast_slice(&view);
            (0..PASS_LABELS.len())
                .map(|i| {
                    let start = timestamps[i * 2];
                    let end = timestamps[i * 2 + 1];
                    end.saturating_sub(start) as f32 * period * 1e-6 // ns → ms
                })
                .collect()
        }; // view dropped here, safe to unmap

        readback.unmap();
        Some(results)
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn labels(&self) -> &[&'static str] {
        PASS_LABELS
    }
}
