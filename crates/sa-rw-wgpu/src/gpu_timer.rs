//! GPU time per frame from timestamp queries (PERF-0, docs/perf/measure.md).
//!
//! Every render pass of a frame writes a timestamp at its start and its end; at the frame's end the used queries are
//! resolved and copied into a read-back buffer that is mapped asynchronously, and the frame's GPU time (the sum of
//! its passes' durations, so idle gaps between passes are not counted) is collected some frames later without
//! waiting. Needs `Features::TIMESTAMP_QUERY` (Vulkan / DX12 / Metal, WebGPU with `timestamp-query`; not WebGL2).

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// Queries per frame (two per render pass); passes beyond that are not timed (the frame is marked partial).
const CAP: u32 = 1024;

struct InFlight {
    tag: u64,
    n: u32,
    buf: wgpu::Buffer,
    /// 0 pending, 1 mapped, 2 failed
    state: Arc<AtomicU8>,
    partial: bool,
}

pub struct GpuTimer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    free: Vec<wgpu::Buffer>,
    inflight: Vec<InFlight>,
    used: u32,
    partial: bool,
    period_ns: f64,
    done: Vec<(u64, f64, bool)>,
}

impl GpuTimer {
    /// A timer on `device`, or `None` without `Features::TIMESTAMP_QUERY`.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<GpuTimer> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("perf timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: CAP,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perf timestamp resolve"),
            size: u64::from(CAP) * 8,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Some(GpuTimer {
            set,
            resolve,
            free: Vec::new(),
            inflight: Vec::new(),
            used: 0,
            partial: false,
            period_ns: f64::from(queue.get_timestamp_period()),
            done: Vec::new(),
        })
    }

    /// The timestamp writes of the next render pass (`None` once the frame's queries are used up).
    pub fn pass(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if self.used + 2 > CAP {
            self.partial = true;
            return None;
        }
        let i = self.used;
        self.used += 2;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: Some(i),
            end_of_pass_write_index: Some(i + 1),
        })
    }

    /// Ends the frame: its queries are resolved and read back under `tag`.
    pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, tag: u64) {
        let n = self.used;
        let partial = self.partial;
        self.used = 0;
        self.partial = false;
        if n == 0 {
            return;
        }
        let buf = self.free.pop().unwrap_or_else(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("perf timestamp read-back"),
                size: u64::from(CAP) * 8,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("perf timestamps"),
        });
        enc.resolve_query_set(&self.set, 0..n, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &buf, 0, u64::from(n) * 8);
        queue.submit([enc.finish()]);
        let state = Arc::new(AtomicU8::new(0));
        let s = state.clone();
        buf.slice(..u64::from(n) * 8)
            .map_async(wgpu::MapMode::Read, move |r| {
                s.store(if r.is_ok() { 1 } else { 2 }, Ordering::SeqCst);
            });
        self.inflight.push(InFlight {
            tag,
            n,
            buf,
            state,
            partial,
        });
    }

    /// Collects the frames whose read-back finished: `(tag, GPU ms, partial)`. With `wait` (end of a run) blocks
    /// until every frame in flight is in.
    pub fn take(&mut self, device: &wgpu::Device, wait: bool) -> Vec<(u64, f64, bool)> {
        if !self.inflight.is_empty() {
            let _ = device.poll(if wait {
                wgpu::PollType::wait_indefinitely()
            } else {
                wgpu::PollType::Poll
            });
        }
        let mut keep = Vec::new();
        for f in std::mem::take(&mut self.inflight) {
            match f.state.load(Ordering::SeqCst) {
                0 => keep.push(f),
                1 => {
                    let ns = {
                        let v = f.buf.slice(..u64::from(f.n) * 8).get_mapped_range();
                        let ts: Vec<u64> = v
                            .as_chunks::<8>()
                            .0
                            .iter()
                            .map(|c| u64::from_le_bytes(*c))
                            .collect();
                        ts.as_chunks::<2>()
                            .0
                            .iter()
                            .map(|p| p[1].saturating_sub(p[0]))
                            .sum::<u64>() as f64
                            * self.period_ns
                    };
                    f.buf.unmap();
                    self.done.push((f.tag, ns / 1e6, f.partial));
                    self.free.push(f.buf);
                }
                _ => {}
            }
        }
        self.inflight = keep;
        std::mem::take(&mut self.done)
    }
}
