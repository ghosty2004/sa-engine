//! Time cycle colours for a time of day and weather (`CTimeCycle::CalcColoursForPoint`,
//! reduced: no time-cycle boxes, no weather interpolation, no smog blending) and the building
//! day/night balance (`CCustomBuildingRenderer::UpdateDayNightBalanceParam`, 0x5D7F80, logic from
//! gta-reversed's commented-out body; not ported/verified).

use sa_formats::data::world::timecyc::{NUM_HOURS, NUM_WEATHERS, TimeCyc};

/// Hours of the 8 time cycle samples (0x8CDECC).
pub const SAMPLE_HOURS: [f32; 9] = [0.0, 5.0, 6.0, 7.0, 12.0, 19.0, 20.0, 22.0, 24.0];

#[derive(Debug, Clone, Copy, Default)]
pub struct Colours {
    pub amb: [f32; 3],
    pub amb_obj: [f32; 3],
    pub dir: [f32; 3],
    pub sky_top: [f32; 3],
    pub sky_bottom: [f32; 3],
    pub far_clip: f32,
    pub fog_start: f32,
    pub water: [f32; 4],
    /// Post effect colours 1 and 2 (rgb 0-1, alpha as stored: `uint8(a * 2)` / 255).
    pub postfx1: [f32; 4],
    pub postfx2: [f32; 4],
}

pub struct TimeCycle {
    /// [weather][hour] samples.
    samples: Vec<[Colours; NUM_HOURS]>,
}

impl TimeCycle {
    pub fn parse(data: &[u8]) -> TimeCycle {
        let tc = TimeCyc::parse(data);
        let mut samples = vec![[Colours::default(); NUM_HOURS]; NUM_WEATHERS];
        for e in &tc.entries {
            let f = |n: &str| e.fields.f32(n).or_else(|| e.fields.i32(n).map(|v| v as f32)).unwrap_or(0.0);
            let c3 = |p: &str| [f(&format!("{p}_r")) / 255.0, f(&format!("{p}_g")) / 255.0, f(&format!("{p}_b")) / 255.0];
            samples[e.weather][e.hour] = Colours {
                amb: c3("amb"),
                amb_obj: c3("amb_obj"),
                dir: c3("dir"),
                sky_top: c3("sky_top"),
                sky_bottom: c3("sky_bot"),
                far_clip: f("far_clip"),
                fog_start: f("fog_start"),
                water: {
                    let w = c3("water");
                    [w[0], w[1], w[2], f("water_a") / 255.0]
                },
                postfx1: {
                    let c = c3("postfx1");
                    [c[0], c[1], c[2], ((f("postfx1_a") * 2.0) as i32 as u8) as f32 / 255.0]
                },
                postfx2: {
                    let c = c3("postfx2");
                    [c[0], c[1], c[2], ((f("postfx2_a") * 2.0) as i32 as u8) as f32 / 255.0]
                },
            };
        }
        TimeCycle { samples }
    }

    pub fn num_weathers(&self) -> usize {
        self.samples.len()
    }

    /// Colours at `hours` (0..24) for `weather`, interpolated between the two samples around it.
    pub fn colours(&self, weather: usize, hours: f32) -> Colours {
        let h = hours.rem_euclid(24.0).min(23.999);
        let mut i = 0;
        while h >= SAMPLE_HOURS[i + 1] {
            i += 1;
        }
        let t = (h - SAMPLE_HOURS[i]) / (SAMPLE_HOURS[i + 1] - SAMPLE_HOURS[i]);
        let s = &self.samples[weather.min(self.samples.len() - 1)];
        let (a, b) = (&s[i], &s[(i + 1) % NUM_HOURS]);
        let l = |x: f32, y: f32| x + (y - x) * t;
        let l3 = |x: [f32; 3], y: [f32; 3]| [l(x[0], y[0]), l(x[1], y[1]), l(x[2], y[2])];
        let l4 = |x: [f32; 4], y: [f32; 4]| [l(x[0], y[0]), l(x[1], y[1]), l(x[2], y[2]), l(x[3], y[3])];
        Colours {
            amb: l3(a.amb, b.amb),
            amb_obj: l3(a.amb_obj, b.amb_obj),
            dir: l3(a.dir, b.dir),
            sky_top: l3(a.sky_top, b.sky_top),
            sky_bottom: l3(a.sky_bottom, b.sky_bottom),
            far_clip: l(a.far_clip, b.far_clip),
            fog_start: l(a.fog_start, b.fog_start),
            water: l4(a.water, b.water),
            postfx1: l4(a.postfx1, b.postfx1),
            postfx2: l4(a.postfx2, b.postfx2),
        }
    }
}

/// `m_fDNBalanceParam` for the minute of the day: 1 before 06:00, fading to 0 by 07:00,
/// 0 until 20:00, back to 1 by 21:00.
pub fn dn_balance(minutes: f32) -> f32 {
    if minutes < 360.0 {
        1.0
    } else if minutes < 420.0 {
        (420.0 - minutes) / 60.0
    } else if minutes < 1200.0 {
        0.0
    } else if minutes >= 1260.0 {
        1.0
    } else {
        1.0 - (1260.0 - minutes) / 60.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dn_balance_ramps() {
        assert_eq!(dn_balance(0.0), 1.0);
        assert_eq!(dn_balance(390.0), 0.5);
        assert_eq!(dn_balance(720.0), 0.0);
        assert_eq!(dn_balance(1230.0), 0.5);
        assert_eq!(dn_balance(1300.0), 1.0);
    }
}
