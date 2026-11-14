//! samp's GUI skin on the engine: `sampgui.png` (512 × 512) and the cursor `mouse.png` (32 × 32, hotspot 0, 0),
//! loose files of the user's SA:MP install loaded at run time by the host (never committed), held in one
//! 512 × 1024 texture (`sampgui.png` at the top, `mouse.png` at (0, 512)) with D3DX's mip chain; the element
//! rectangles are samp-ui's (`samp_ui::dxut::skin`). Without the files the controls are flat rectangles of the
//! same colours and the cursor a drawn arrow.

use super::draw::{Cmd, DrawList};

/// Width of the combined skin texture.
pub const TEXTURE_W: u32 = 512;
/// Height of the combined skin texture.
pub const TEXTURE_H: u32 = 1024;
/// Where `mouse.png` sits in the combined texture.
pub const CURSOR_SRC: [f32; 4] = [0.0, 512.0, 32.0, 544.0];

/// The mip chain of an RGBA8 image as `D3DXCreateTextureFromFileEx` builds it for samp's skin (D3DX_DEFAULT
/// levels: down to 1 × 1; D3DX_DEFAULT mip filter: box, each texel the rounded mean of the 2 × 2 texels above
/// it, channels straight). Level 0 first.
pub fn mip_chain(w: u32, h: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut out = vec![(w, h, rgba[..(w * h * 4) as usize].to_vec())];
    while let Some((pw, ph, p)) = out.last() {
        let (pw, ph) = (*pw, *ph);
        if pw == 1 && ph == 1 {
            break;
        }
        let (nw, nh) = ((pw / 2).max(1), (ph / 2).max(1));
        let mut n = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let at = |xx: u32, yy: u32| u32::from(p[((yy.min(ph - 1) * pw + xx.min(pw - 1)) * 4 + c) as usize]);
                    let (x0, y0) = (2 * x, 2 * y);
                    let sum = at(x0, y0) + at(x0 + 1, y0) + at(x0, y0 + 1) + at(x0 + 1, y0 + 1);
                    n[((y * nw + x) * 4 + c) as usize] = ((sum + 2) / 4) as u8;
                }
            }
        }
        out.push((nw, nh, n));
    }
    out
}

/// The mouse cursor at `(x, y)`: `mouse.png` from the skin, else a drawn arrow (a black arrow with a white
/// inner arrow).
pub fn cursor(d: &mut DrawList, skin: bool, x: f32, y: f32) {
    if skin {
        d.cmds.push(Cmd::Skin { src: CURSOR_SRC, dst: [x, y, x + 32.0, y + 32.0], colour: 0xFFFF_FFFF });
        return;
    }
    let arrow = |s: f32, o: f32, colour: u32, d: &mut DrawList| {
        let p = |dx: f32, dy: f32| (x + o + dx * s, y + o * 2.0 + dy * s);
        d.cmds.push(Cmd::Triangle { pts: [p(0.0, 0.0), p(0.0, 16.0), p(11.0, 11.0)], colour });
        d.cmds.push(Cmd::Triangle { pts: [p(3.0, 11.0), p(5.5, 9.0), p(8.0, 18.0)], colour });
    };
    arrow(1.15, -1.0, 0xFF00_0000, d);
    arrow(1.0, 0.0, 0xFFFF_FFFF, d);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_box_filters_down_to_one_texel() {
        let mut img = vec![0u8; 4 * 4 * 4];
        for (i, p) in img.chunks_mut(4).enumerate() {
            p.copy_from_slice(&[(i * 10) as u8, 255, 0, if i % 2 == 0 { 255 } else { 0 }]);
        }
        let m = mip_chain(4, 4, &img);
        assert_eq!(m.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(), [(4, 4), (2, 2), (1, 1)]);
        // Texel (0, 0) of level 1 = the mean of texels 0, 1, 4, 5: red (0 + 10 + 40 + 50) / 4 = 25, alpha 127.5 → 128.
        assert_eq!(&m[1].2[..4], &[25, 255, 0, 128]);
        assert_eq!(mip_chain(512, 1024, &vec![0; 512 * 1024 * 4]).len(), 11);
    }

    #[test]
    fn cursor_from_skin_or_arrow() {
        let mut d = DrawList::default();
        cursor(&mut d, true, 10.0, 20.0);
        assert_eq!(d.cmds, vec![Cmd::Skin { src: CURSOR_SRC, dst: [10.0, 20.0, 42.0, 52.0], colour: 0xFFFF_FFFF }]);
        let mut d = DrawList::default();
        cursor(&mut d, false, 10.0, 20.0);
        assert_eq!(d.cmds.len(), 4);
    }
}
