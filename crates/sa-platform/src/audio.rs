//! Audio output: the sink the phase B mixer writes its mixed buffers into.
//!
//! The original plays through DirectSound: `CAEAudioHardware` owns DirectSound secondary buffers
//! (one per `CAEAudioChannel`) and DirectSound mixes them into the primary buffer. Phase B has no
//! DirectSound, so the channel layer (`CAEAudioChannel`/`CAEStaticChannel`/`CAEStreamingChannel`
//! once ported) feeds a software mixer, and the mixer pushes interleaved PCM here. [`AudioOut`] is
//! that sink: `push` never blocks, [`AudioOut::queued_frames`] tells the mixer how far ahead it is
//! (DirectSound's play/write cursor distance), and [`AudioOut::frames_played`] is the play
//! cursor, which the stream thread uses to pace decoding.
//!
//! [`NullAudio`] consumes everything (headless runs, CI, determinism mode); the native backend is
//! `native::CpalAudio` (feature `audio`).

/// Sample format of a sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioFormat {
    /// What the game asks DirectSound for (`CAEAudioHardware::Initialise`: 44.1 kHz stereo,
    /// 16-bit primary buffer).
    pub const GAME: AudioFormat = AudioFormat {
        sample_rate: 44_100,
        channels: 2,
    };
}

/// A PCM sink (interleaved signed 16-bit, the format of DirectSound buffers).
pub trait AudioOut: Send {
    fn format(&self) -> AudioFormat;
    /// Queues interleaved frames; never blocks. Returns the frames accepted (a full queue drops
    /// the excess, like writing past DirectSound's play cursor would overwrite unplayed data).
    fn push(&mut self, interleaved: &[i16]) -> usize;
    /// Frames queued but not yet played.
    fn queued_frames(&self) -> usize;
    /// Frames played since creation (the play cursor).
    fn frames_played(&self) -> u64;
}

/// A sink that plays nothing: every pushed frame counts as played at once.
#[derive(Debug, Default)]
pub struct NullAudio {
    played: u64,
}

impl AudioOut for NullAudio {
    fn format(&self) -> AudioFormat {
        AudioFormat::GAME
    }
    fn push(&mut self, interleaved: &[i16]) -> usize {
        let frames = interleaved.len() / AudioFormat::GAME.channels as usize;
        self.played += frames as u64;
        frames
    }
    fn queued_frames(&self) -> usize {
        0
    }
    fn frames_played(&self) -> u64 {
        self.played
    }
}

/// Converts interleaved frames between channel counts (mono ↔ stereo, extra channels silent),
/// for devices whose native layout differs from [`AudioFormat::GAME`].
pub fn remix(src: &[i16], from: u16, to: u16, out: &mut Vec<i16>) {
    let (from, to) = (from.max(1) as usize, to.max(1) as usize);
    for frame in src.chunks_exact(from) {
        for c in 0..to {
            out.push(match (from, c) {
                (1, _) if c < 2 => frame[0],
                (_, c) if c < from && (to > 1 || from == 1) => frame[c],
                // stereo → mono: average
                (_, 0) => ((frame[0] as i32 + frame[1] as i32) / 2) as i16,
                _ => 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_sink_counts_frames() {
        let mut a = NullAudio::default();
        assert_eq!(a.push(&[0; 8]), 4);
        assert_eq!(a.frames_played(), 4);
    }

    #[test]
    fn remix_layouts() {
        let mut v = Vec::new();
        remix(&[1, 2, 3], 1, 2, &mut v);
        assert_eq!(v, [1, 1, 2, 2, 3, 3]);
        v.clear();
        remix(&[10, 20, -4, 4], 2, 1, &mut v);
        assert_eq!(v, [15, 0]);
        v.clear();
        remix(&[1, 2], 2, 4, &mut v);
        assert_eq!(v, [1, 2, 0, 0]);
    }
}
