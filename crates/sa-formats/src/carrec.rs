//! Vehicle recordings (`data/Paths/carrec.img`, entries `carrecNNN.rrr`).
//!
//! Loader: `CVehicleRecording::Load` 0x45A8F0 (stub; body at 0x156F510 in `.HOODLUM`). It reads
//! the whole streamed entry (`RwStreamRead(stream, buf, 9999999)`, i.e. every sector of the IMG
//! entry) and then walks it in 0x20-byte steps: the first frame at index > 0 whose time word is 0
//! cuts the recording there (the zero IMG sector padding ends it). `SmoothRecording` 0x45A0F0
//! then rewrites frame times at load; the parser keeps the file values.
//!
//! Frame (`CVehicleStateEachFrame`, 0x20 bytes), with the scales the exe applies
//! (`RestoreInfoForMatrix` 0x459960, `RestoreInfoForCar` 0x459A30):
//!
//! | off | type | field | scale |
//! |---|---|---|---|
//! | 0x00 | `u32` | time (ms) | |
//! | 0x04 | `i16[3]` | move speed | `* 0x38800100` (f32 nearest to 1/16383.5) |
//! | 0x0A | `i8[3]` | matrix right | `* 0x3C010204` (1/127) |
//! | 0x0D | `i8[3]` | matrix up | 1/127 |
//! | 0x10 | `i8` | steering angle | `* 0x3D4CCCCD` (0.05); read with `movsx` (signed) |
//! | 0x11 | `i8` | gas pedal | `* 0x3C23D70A` (0.01); signed |
//! | 0x12 | `i8` | brake pedal | 0.01; signed |
//! | 0x13 | `u8` | handbrake (bool) | |
//! | 0x14 | `f32[3]` | position | |

use crate::bytes::{Put, Reader};
use crate::error::Result;

/// Size of one frame.
pub const FRAME_SIZE: usize = 0x20;

/// One recorded frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub time: u32,
    pub velocity: [i16; 3],
    pub right: [i8; 3],
    pub up: [i8; 3],
    pub steer: i8,
    pub gas: i8,
    pub brake: i8,
    pub handbrake: u8,
    pub pos: [f32; 3],
}

impl Frame {
    fn read(r: &mut Reader<'_>) -> Result<Frame> {
        let time = r.u32()?;
        let velocity = [r.i16()?, r.i16()?, r.i16()?];
        let b = r.array::<6>()?;
        let c = r.array::<4>()?;
        Ok(Frame {
            time,
            velocity,
            right: [b[0] as i8, b[1] as i8, b[2] as i8],
            up: [b[3] as i8, b[4] as i8, b[5] as i8],
            steer: c[0] as i8,
            gas: c[1] as i8,
            brake: c[2] as i8,
            handbrake: c[3],
            pos: r.f32s::<3>()?,
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.put_u32(self.time);
        for v in self.velocity {
            out.put_i16(v);
        }
        for v in self.right.iter().chain(&self.up) {
            out.push(*v as u8);
        }
        out.extend_from_slice(&[self.steer as u8, self.gas as u8, self.brake as u8, self.handbrake]);
        out.put_f32s(&self.pos);
    }

    /// Velocity as the exe restores it (`int * f32 constant`, one rounding to `f32`).
    pub fn velocity_f32(&self) -> [f32; 3] {
        let c = f64::from(f32::from_bits(0x3880_0100));
        self.velocity.map(|v| (f64::from(v) * c) as f32)
    }
}

/// A parsed recording.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recording {
    /// The frames the exe keeps.
    pub frames: Vec<Frame>,
    /// The rest of the entry: from the cutting zero-time frame (or the incomplete last 0x20
    /// bytes) to the end.
    pub trailing: Vec<u8>,
}

impl Recording {
    /// Parses an entry the way `CVehicleRecording::Load` cuts it.
    pub fn parse(data: &[u8]) -> Result<Recording> {
        let mut r = Reader::new(data, "carrec frame");
        let mut frames = Vec::new();
        while r.remaining() >= FRAME_SIZE {
            let mut peek = r.clone();
            let f = Frame::read(&mut peek)?;
            if !frames.is_empty() && f.time == 0 {
                break;
            }
            frames.push(f);
            r = peek;
        }
        Ok(Recording {
            frames,
            trailing: r.rest().to_vec(),
        })
    }

    /// Serialises back to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.frames.len() * FRAME_SIZE + self.trailing.len());
        for f in &self.frames {
            f.write(&mut out);
        }
        out.extend_from_slice(&self.trailing);
        out
    }

    /// Duration (time of the last frame).
    pub fn duration(&self) -> u32 {
        self.frames.last().map_or(0, |f| f.time)
    }

    /// Report statistics: `(label, count)`.
    pub fn notes(&self) -> Vec<(String, u64)> {
        let mut v = Vec::new();
        if self.trailing.iter().any(|&b| b != 0) {
            v.push(("entries with non-zero bytes after the last kept frame".into(), 1));
        }
        if self.frames.windows(2).any(|w| w[1].time < w[0].time) {
            v.push(("entries with decreasing frame times".into(), 1));
        }
        if self.frames.first().is_some_and(|f| f.time != 0) {
            v.push(("entries whose first frame time is not 0".into(), 1));
        }
        if self.frames.iter().any(|f| f.handbrake > 1) {
            v.push(("entries with handbrake byte > 1".into(), 1));
        }
        if self.frames.iter().any(|f| f.gas < 0 || f.brake < 0) {
            v.push((
                "entries with negative gas/brake bytes (exe reads them signed)".into(),
                1,
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_at_zero_time_and_round_trips() {
        let mut fs = vec![Frame::default(); 3];
        fs[1].time = 100;
        fs[1].velocity = [16383, -1, 0];
        fs[1].steer = -20;
        fs[1].pos = [1.0, 2.0, 3.0];
        fs[2].time = 200;
        let mut b = Vec::new();
        for f in &fs {
            f.write(&mut b);
        }
        b.extend_from_slice(&[0; 40]); // padding: a zero-time frame + 8 bytes
        let r = Recording::parse(&b).unwrap();
        assert_eq!(r.frames, fs);
        assert_eq!(r.trailing.len(), 40);
        assert_eq!(r.to_bytes(), b);
        assert_eq!(r.duration(), 200);
        assert!((r.frames[1].velocity_f32()[0] - 1.0).abs() < 1e-4);
        assert!(r.notes().is_empty());
    }
}
