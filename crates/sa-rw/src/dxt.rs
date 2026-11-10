//! Software DXT1/DXT3/DXT5 (BC1/BC2/BC3) decoder, for GPUs or APIs without BC formats.
//!
//! Colour endpoints expand 5/6-bit channels by bit replication; DXT1 uses the 3-colour +
//! transparent black mode when `c0 <= c1` (D3D9 behaviour).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bc1,
    Bc2,
    Bc3,
}

fn rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 31) as u8;
    let g = ((c >> 5) & 63) as u8;
    let b = (c & 31) as u8;
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

fn colour_block(b: &[u8], dxt1: bool, out: &mut [[u8; 4]; 16]) {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let mix = |a: u8, b: u8, wa: u16, wb: u16, d: u16| ((a as u16 * wa + b as u16 * wb) / d) as u8;
    let mut pal = [[0u8; 4]; 4];
    pal[0] = [p0[0], p0[1], p0[2], 255];
    pal[1] = [p1[0], p1[1], p1[2], 255];
    if !dxt1 || c0 > c1 {
        for k in 0..3 {
            pal[2][k] = mix(p0[k], p1[k], 2, 1, 3);
            pal[3][k] = mix(p0[k], p1[k], 1, 2, 3);
        }
        pal[2][3] = 255;
        pal[3][3] = 255;
    } else {
        for k in 0..3 {
            pal[2][k] = mix(p0[k], p1[k], 1, 1, 2);
        }
        pal[2][3] = 255;
        pal[3] = [0, 0, 0, 0];
    }
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    for (i, px) in out.iter_mut().enumerate() {
        *px = pal[((bits >> (2 * i)) & 3) as usize];
    }
}

/// Decodes `w`×`h` pixels of block data into RGBA8. Missing trailing blocks decode as zero.
pub fn decode(kind: Kind, data: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let bw = w.div_ceil(4);
    let bh = h.div_ceil(4);
    let bsize = if kind == Kind::Bc1 { 8 } else { 16 };
    let mut out = vec![0u8; w * h * 4];
    let mut px = [[0u8; 4]; 16];
    for by in 0..bh {
        for bx in 0..bw {
            let off = (by * bw + bx) * bsize;
            let Some(b) = data.get(off..off + bsize) else { continue };
            match kind {
                Kind::Bc1 => colour_block(b, true, &mut px),
                Kind::Bc2 => {
                    colour_block(&b[8..], false, &mut px);
                    for (i, p) in px.iter_mut().enumerate() {
                        let a = (b[i / 2] >> (4 * (i % 2))) & 15;
                        p[3] = a * 17;
                    }
                }
                Kind::Bc3 => {
                    colour_block(&b[8..], false, &mut px);
                    let (a0, a1) = (b[0] as u16, b[1] as u16);
                    let mut ap = [0u8; 8];
                    ap[0] = a0 as u8;
                    ap[1] = a1 as u8;
                    if a0 > a1 {
                        for k in 1..7u16 {
                            ap[k as usize + 1] = (((7 - k) * a0 + k * a1) / 7) as u8;
                        }
                    } else {
                        for k in 1..5u16 {
                            ap[k as usize + 1] = (((5 - k) * a0 + k * a1) / 5) as u8;
                        }
                        ap[6] = 0;
                        ap[7] = 255;
                    }
                    let mut bits = 0u64;
                    for k in 0..6 {
                        bits |= (b[2 + k] as u64) << (8 * k);
                    }
                    for (i, p) in px.iter_mut().enumerate() {
                        p[3] = ap[((bits >> (3 * i)) & 7) as usize];
                    }
                }
            }
            for (i, p) in px.iter().enumerate() {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x < w && y < h {
                    out[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(p);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bc1_solid_and_transparent() {
        // c0 = red, c1 = blue, c0 > c1, all indices 0 → red
        let mut b = Vec::new();
        b.extend_from_slice(&0xF800u16.to_le_bytes());
        b.extend_from_slice(&0x001Fu16.to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0]);
        let px = decode(Kind::Bc1, &b, 4, 4);
        assert_eq!(&px[..4], &[255, 0, 0, 255]);
        // c0 <= c1 and index 3 → transparent black
        let mut b = Vec::new();
        b.extend_from_slice(&0x001Fu16.to_le_bytes());
        b.extend_from_slice(&0xF800u16.to_le_bytes());
        b.extend_from_slice(&[0xFF; 4]);
        let px = decode(Kind::Bc1, &b, 2, 2);
        assert_eq!(px, vec![0; 16]);
    }

    #[test]
    fn bc3_alpha_interpolates() {
        let mut b = vec![255, 0, 0b0000_0010, 0, 0, 0, 0, 0];
        b.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]);
        let px = decode(Kind::Bc3, &b, 4, 4);
        // pixel 0 has alpha index 2 → (6*255 + 1*0) / 7
        assert_eq!(px[3], (6 * 255 / 7) as u8);
        assert_eq!(px[7], 255);
    }
}
