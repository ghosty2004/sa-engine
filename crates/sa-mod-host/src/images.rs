//! The mods' overlay images ([`Images`]: one mod's images within its budget, `sa_mod_api::image`) and the clipping
//! of overlay quads ([`clip`]).

use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use sa_mod_api::image::{MAX_BYTES, MAX_IMAGES, MAX_SIDE};
use sa_platform::gpu::{OverlayImage, OverlayQuad};

/// The renderer's id of the next image (never 0, never reused, unique over every mod).
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// One mod's images (a handle is the index of its slot) and the changes the renderer has not taken yet.
#[derive(Debug, Default)]
pub struct Images {
    /// Per slot: the renderer's id and the image's bytes.
    slots: Vec<Option<(u32, usize)>>,
    bytes: usize,
    added: Vec<OverlayImage>,
    removed: Vec<u32>,
}

/// Why an image was not created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateError {
    /// A side of 0 or above [`MAX_SIDE`], or pixel data of the wrong length.
    BadSize,
    /// The mod holds [`MAX_IMAGES`] images or [`MAX_BYTES`] bytes.
    Budget,
}

impl Images {
    /// The images the mod holds.
    pub fn count(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Takes a `width` x `height` RGBA8 image: its handle.
    pub fn create(
        &mut self,
        width: u32,
        height: u32,
        smooth: bool,
        rgba: &[u8],
    ) -> Result<usize, CreateError> {
        if !(1..=MAX_SIDE).contains(&width) || !(1..=MAX_SIDE).contains(&height) {
            return Err(CreateError::BadSize);
        }
        let len = width as usize * height as usize * 4;
        if rgba.len() != len {
            return Err(CreateError::BadSize);
        }
        if self.count() >= MAX_IMAGES || self.bytes + len > MAX_BYTES {
            return Err(CreateError::Budget);
        }
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let i = match self.slots.iter().position(Option::is_none) {
            Some(i) => i,
            None => {
                self.slots.push(None);
                self.slots.len() - 1
            }
        };
        self.slots[i] = Some((id, len));
        self.bytes += len;
        self.added.push(OverlayImage {
            id,
            width,
            height,
            smooth,
            rgba: Rc::from(rgba),
        });
        Ok(i)
    }

    /// The renderer's id of `handle`.
    pub fn id(&self, handle: i32) -> Option<u32> {
        Some(self.slots.get(usize::try_from(handle).ok()?)?.as_ref()?.0)
    }

    /// Deletes `handle`; false when there is none.
    pub fn destroy(&mut self, handle: i32) -> bool {
        let Some((id, len)) = usize::try_from(handle)
            .ok()
            .and_then(|i| self.slots.get_mut(i))
            .and_then(Option::take)
        else {
            return false;
        };
        self.bytes -= len;
        // an image created and deleted before the renderer saw it never reaches it
        match self.added.iter().position(|a| a.id == id) {
            Some(p) => {
                self.added.remove(p);
            }
            None => self.removed.push(id),
        }
        true
    }

    /// Deletes every image (the mod was disabled).
    pub fn clear(&mut self) {
        for h in 0..self.slots.len() {
            self.destroy(h as i32);
        }
    }

    /// Moves the changes since the last call to the renderer's lists.
    pub fn take_changes(&mut self, added: &mut Vec<OverlayImage>, removed: &mut Vec<u32>) {
        added.append(&mut self.added);
        removed.append(&mut self.removed);
    }
}

/// `q` cut to the rectangle `clip` (`[x, y, w, h]`), its texture coordinates with it; `None` when nothing is left.
pub fn clip(q: OverlayQuad, clip: Option<[f32; 4]>) -> Option<OverlayQuad> {
    if q.w <= 0.0 || q.h <= 0.0 {
        return None;
    }
    let Some([cx, cy, cw, ch]) = clip else {
        return Some(q);
    };
    let (x0, y0) = (q.x.max(cx), q.y.max(cy));
    let (x1, y1) = ((q.x + q.w).min(cx + cw), (q.y + q.h).min(cy + ch));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let [u0, v0, u1, v1] = q.uv;
    let u = |x: f32| u0 + (u1 - u0) * (x - q.x) / q.w;
    let v = |y: f32| v0 + (v1 - v0) * (y - q.y) / q.h;
    Some(OverlayQuad {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
        uv: [u(x0), v(y0), u(x1), v(y1)],
        ..q
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_handles_and_changes() {
        let mut im = Images::default();
        assert_eq!(im.create(0, 4, false, &[]), Err(CreateError::BadSize));
        assert_eq!(im.create(2, 2, false, &[0; 15]), Err(CreateError::BadSize));
        let a = im.create(2, 2, true, &[1; 16]).unwrap();
        let b = im.create(1, 1, false, &[2; 4]).unwrap();
        assert_eq!((a, b), (0, 1));
        let (ida, idb) = (im.id(0).unwrap(), im.id(1).unwrap());
        assert_ne!(ida, idb);
        // b is deleted before the renderer saw it: only a reaches it
        assert!(im.destroy(1) && !im.destroy(1));
        let (mut added, mut removed) = (Vec::new(), Vec::new());
        im.take_changes(&mut added, &mut removed);
        assert_eq!(added.iter().map(|i| i.id).collect::<Vec<_>>(), [ida]);
        assert!(removed.is_empty());
        im.clear();
        im.take_changes(&mut added, &mut removed);
        assert_eq!(removed, [ida]);
        // the byte budget
        let side = MAX_SIDE / 2;
        let big = vec![0; (side * side * 4) as usize];
        assert_eq!(big.len() * 2, MAX_BYTES);
        assert!(im.create(side, side, false, &big).is_ok());
        assert!(im.create(side, side, false, &big).is_ok());
        assert_eq!(im.create(side, side, false, &big), Err(CreateError::Budget));
    }

    #[test]
    fn clipping_cuts_texture_coordinates_too() {
        let q = OverlayQuad {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            uv: [0.0, 0.0, 1.0, 0.5],
            rgba: 1,
            image: 3,
        };
        assert_eq!(clip(q, None), Some(q));
        let c = clip(q, Some([5.0, -5.0, 100.0, 7.0])).unwrap();
        assert_eq!((c.x, c.y, c.w, c.h), (5.0, 0.0, 5.0, 2.0));
        assert_eq!(c.uv, [0.5, 0.0, 1.0, 0.1]);
        assert_eq!(clip(q, Some([20.0, 0.0, 5.0, 5.0])), None);
    }
}
