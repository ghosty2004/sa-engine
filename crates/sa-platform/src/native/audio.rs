//! cpal output device behind [`AudioOut`].
//!
//! The device callback pulls from a ring buffer that [`AudioOut::push`] fills; underruns play
//! silence. Pushed frames are in [`AudioFormat::GAME`] (44.1 kHz stereo i16); the device runs at
//! its own default rate and channel count, so the callback converts channels ([`remix`]) and
//! resamples linearly (the original lets DirectSound do both).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::{AudioFormat, AudioOut, remix};

struct Shared {
    queue: Mutex<VecDeque<i16>>,
    played: AtomicU64,
}

/// The default output device.
pub struct CpalAudio {
    shared: Arc<Shared>,
    /// Upper bound of queued frames (about 250 ms).
    capacity: usize,
    /// Dropping it ends the thread that owns the stream.
    _stop: mpsc::Sender<()>,
}

impl CpalAudio {
    /// Opens the default output device. `cpal::Stream` is not `Send` on every platform, so the
    /// stream lives on its own thread for the sink's lifetime.
    pub fn new() -> Result<CpalAudio, String> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(VecDeque::new()),
            played: AtomicU64::new(0),
        });
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let s = shared.clone();
        std::thread::Builder::new()
            .name("sa-audio-out".into())
            .spawn(move || match open_stream(s) {
                Ok(stream) => {
                    let _ = ready_tx.send(Ok(()));
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| e.to_string())?;
        ready_rx.recv().map_err(|e| e.to_string())??;
        Ok(CpalAudio {
            shared,
            capacity: AudioFormat::GAME.sample_rate as usize / 4,
            _stop: stop_tx,
        })
    }
}

fn open_stream(s: Arc<Shared>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let dev = host.default_output_device().ok_or("no audio output device")?;
    let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
    let dev_channels = cfg.channels();
    let dev_rate = cfg.sample_rate();
    let src = AudioFormat::GAME;
    let step = src.sample_rate as f64 / dev_rate as f64;
    let mut phase = 0.0f64;
    let mut prev = [0i16; 2];
    let mut cur = [0i16; 2];
    let mut tmp = Vec::new();
    let mut mixed = Vec::new();
    let stream = dev
        .build_output_stream(
            &cfg.config(),
            move |out: &mut [f32], _| {
                let frames = out.len() / dev_channels.max(1) as usize;
                let mut consumed = 0u64;
                tmp.clear();
                {
                    let mut q = s.queue.lock().unwrap_or_else(|e| e.into_inner());
                    for _ in 0..frames {
                        phase += step;
                        while phase >= 1.0 {
                            phase -= 1.0;
                            prev = cur;
                            if q.len() >= 2 {
                                cur = [q.pop_front().unwrap_or(0), q.pop_front().unwrap_or(0)];
                                consumed += 1;
                            } else {
                                cur = [0, 0];
                            }
                        }
                        for c in 0..2 {
                            let v = prev[c] as f64 + (cur[c] as f64 - prev[c] as f64) * phase;
                            tmp.push(v as i16);
                        }
                    }
                }
                s.played.fetch_add(consumed, Ordering::Relaxed);
                mixed.clear();
                remix(&tmp, 2, dev_channels, &mut mixed);
                for (o, v) in out.iter_mut().zip(mixed.iter().chain(std::iter::repeat(&0))) {
                    *o = *v as f32 / 32768.0;
                }
            },
            |e| eprintln!("sa-platform audio: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

impl AudioOut for CpalAudio {
    fn format(&self) -> AudioFormat {
        AudioFormat::GAME
    }
    fn push(&mut self, interleaved: &[i16]) -> usize {
        let mut q = self.shared.queue.lock().unwrap_or_else(|e| e.into_inner());
        let room = self.capacity.saturating_sub(q.len() / 2);
        let frames = (interleaved.len() / 2).min(room);
        q.extend(&interleaved[..frames * 2]);
        frames
    }
    fn queued_frames(&self) -> usize {
        self.shared.queue.lock().unwrap_or_else(|e| e.into_inner()).len() / 2
    }
    fn frames_played(&self) -> u64 {
        self.shared.played.load(Ordering::Relaxed)
    }
}
