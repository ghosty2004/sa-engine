//! Builds the object model from every DFF and TXD in `models/gta3.img` and `models/gta_int.img`.
//! Needs the game data: `SA_GAME_DIR=/path/to/gta-sa cargo test -p sa-rw -- --ignored`.

use std::path::{Path, PathBuf};

use sa_formats::img::ImgDirectory;

fn game_dir() -> PathBuf {
    let dir = std::env::var_os("SA_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wine/gta-sa"));
    assert!(dir.join("models").is_dir(), "game data not found at {} (set SA_GAME_DIR)", dir.display());
    dir
}

fn winding(g: &sa_rw::Geometry, idx: &[u32], out: &mut [usize; 2]) {
    let Some(n) = &g.normals else { return };
    let p = &g.positions;
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    for t in idx.as_chunks::<3>().0 {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let (u, v) = (sub(p[b], p[a]), sub(p[c], p[a]));
        let f = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let s: f32 = (0..3).map(|k| f[k] * (n[a][k] + n[b][k] + n[c][k])).sum();
        if s > 0.0 {
            out[0] += 1;
        } else if s < 0.0 {
            out[1] += 1;
        }
    }
}

#[test]
#[ignore = "needs the game data (SA_GAME_DIR)"]
fn every_model_and_dictionary_builds() {
    let dir = game_dir();
    for img in ["gta3.img", "gta_int.img"] {
        let data = std::fs::read(dir.join("models").join(img)).unwrap();
        let d = ImgDirectory::parse(&data).unwrap();
        let (mut dffs, mut txds, mut textures, mut meshes, mut night) = (0, 0, 0, 0, 0);
        let mut failures = Vec::new();
        let mut wind = [0usize; 2];
        for e in &d.entries {
            let name = e.name_lossy().to_ascii_lowercase();
            let bytes = d.entry_data(&data, e).unwrap();
            if name.ends_with(".dff") {
                match sa_rw::read_clump(bytes) {
                    Ok(c) => {
                        dffs += 1;
                        for a in &c.atomics {
                            meshes += a.geometry.meshes.len();
                            night += a.geometry.night_colors.is_some() as usize;
                            for m in &a.geometry.meshes {
                                assert!(m.indices.len() % 3 == 0, "{name}");
                                winding(&a.geometry, &m.indices, &mut wind);
                            }
                        }
                    }
                    Err(e) => failures.push(format!("{name}: {e}")),
                }
            } else if name.ends_with(".txd") {
                match sa_rw::read_tex_dictionary(bytes) {
                    Ok(t) => {
                        txds += 1;
                        textures += t.textures.len();
                        for t in &t.textures {
                            // every level decodes to the expected RGBA size
                            let px = t.raster.level_rgba8(0);
                            assert_eq!(px.len() as u32, t.raster.width * t.raster.height * 4, "{name}/{}", t.name);
                        }
                    }
                    Err(e) => failures.push(format!("{name}: {e}")),
                }
            }
        }
        // Winding convention of the triangle lists relative to the stored vertex normals:
        // [counter-clockwise (right-handed cross product along the normal), clockwise].
        println!("{img}: winding vs normals: {} ccw, {} cw", wind[0], wind[1]);
        println!("{img}: {dffs} clumps ({meshes} meshes, {night} with night colours), {txds} dictionaries ({textures} textures)");
        assert!(failures.is_empty(), "{img}: {} failures, first: {:?}", failures.len(), &failures[..failures.len().min(10)]);
    }
}
