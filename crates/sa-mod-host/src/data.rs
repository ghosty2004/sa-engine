//! Data mods (ADR-004 §4.2): game data files a mod adds from its own folder, loaded at start-up the way the game
//! loads its own.
//!
//! ```toml
//! [data]
//! img = ["SAMP.img"]          # image archives: models, textures, collision, animations (IFP), streamed IPLs
//! ide = ["SAMP.ide"]          # model definitions
//! col = ["extra.col"]         # collision files
//! ipl = ["SAMP.ipl"]          # placements
//! txd = ["blanktex.txd"]      # texture dictionaries, each in a slot named after its file ("blanktex")
//! ```
//!
//! A mod's image archives are joined into one (`mods\<id>\archives.img`, [`Joined`]: one directory, the archives'
//! bytes after it), so a mod takes one of the game's image slots. Images, collision files, definitions and
//! placements become lines of the level file the game reads
//! (`DATA\GTA.DAT`, [`merge_level_file`]): `IMG`, `COLFILE 0` and `IDE` lines before its first `IPL` line (the
//! game registers its images and reads its definitions there, before the first placement sets up the model
//! strings, the streaming directory and the animation blocks), `IPL` lines after its own. An image's directory
//! registers what it holds like the game's own images do: models by name, texture dictionaries, collision and IPL
//! slots, and every `.ifp` as an animation block. Texture dictionaries are loaded into their slots where the game
//! loads its own loose dictionaries (after `CGame::Init1`'s "particle" slot; the host does it before `CGame::Init2`).
//!
//! The game's limits hold: eight images in all (the game's own take two from `CStreaming::InitImageList`, one for
//! `MODELS\PLAYER.IMG` and one per `IMG` line of its level files, six with the stock files), model ids below 20,000, and each model-info
//! store sized for what the mods' definitions add ([`StoreNeeds`]). A mod's data loads completely or not at all:
//! a missing file, a bad definition or no room leaves it out and the log says why; the mod itself still runs.

use std::fmt;

use std::sync::Arc;

use sa_platform::fs::{GameFs, RandomRead};
use toml::Value;

/// The kinds of files a mod's `[data]` names, by their manifest key (also their file extension).
pub const KINDS: [&str; 5] = ["img", "ide", "col", "ipl", "txd"];

/// The longest file name a mod's `[data]` may name (an image directory's name field: 24 bytes with its NUL).
pub const MAX_NAME: usize = 23;

/// `CStreaming::ms_files`: the image archives the game can open.
pub const MAX_IMAGES: usize = 8;
/// The images the game opens besides its level files' `IMG` lines: `MODELS\GTA3.IMG` and `MODELS\GTA_INT.IMG`
/// (`CStreaming::InitImageList`) and `MODELS\PLAYER.IMG` (`CClothes::Init`).
pub const GAME_IMAGES: usize = 3;
/// The longest image path `CStreaming::AddImageToList` keeps (a 40-byte name with its NUL).
pub const MAX_IMAGE_PATH: usize = 39;
/// The name of a mod's joined image archive in its folder (not a name `[data]` accepts: two dots).
pub const ARCHIVES: &str = "archives.all.img";
/// An image archive's sector.
const SECTOR: u64 = 2048;

/// The files of a mod's `[data]` table, as the manifest names them (each in the mod's folder).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataFiles {
    /// Image archives.
    pub img: Vec<String>,
    /// Model definitions.
    pub ide: Vec<String>,
    /// Collision files.
    pub col: Vec<String>,
    /// Placements.
    pub ipl: Vec<String>,
    /// Texture dictionaries.
    pub txd: Vec<String>,
}

impl DataFiles {
    /// Whether it names no file.
    pub fn is_empty(&self) -> bool {
        self.all().next().is_none()
    }

    /// Every file it names.
    pub fn all(&self) -> impl Iterator<Item = &String> {
        self.img
            .iter()
            .chain(&self.ide)
            .chain(&self.col)
            .chain(&self.ipl)
            .chain(&self.txd)
    }

    fn list_mut(&mut self, kind: &str) -> Option<&mut Vec<String>> {
        Some(match kind {
            "img" => &mut self.img,
            "ide" => &mut self.ide,
            "col" => &mut self.col,
            "ipl" => &mut self.ipl,
            "txd" => &mut self.txd,
            _ => return None,
        })
    }

    /// Parses the `[data]` table: lists of plain file names whose extension is their key. Unknown keys are
    /// warnings.
    pub fn parse(table: &Value, warnings: &mut Vec<String>) -> Result<DataFiles, String> {
        let Value::Table(t) = table else {
            return Err("mod.toml: [data] is not a table".into());
        };
        let mut files = DataFiles::default();
        for (k, v) in t {
            let Some(list) = files.list_mut(k) else {
                warnings.push(format!(
                    "[data] {k}: unknown kind, ignored (known: {})",
                    KINDS.join(", ")
                ));
                continue;
            };
            let bad =
                |v: &Value| format!("mod.toml: data.{k}: expected a list of .{k} file names, got {v:?}");
            let Value::Array(names) = v else {
                return Err(bad(v));
            };
            for n in names {
                let Value::String(n) = n else { return Err(bad(n)) };
                check_name(n, k).map_err(|e| format!("mod.toml: data.{k}: {e}"))?;
                if list.iter().any(|m| m.eq_ignore_ascii_case(n)) {
                    return Err(format!("mod.toml: data.{k}: {n:?} is named twice"));
                }
                list.push(n.clone());
            }
        }
        Ok(files)
    }
}

/// A plain file name in the mod's folder with the extension `.kind`: letters, digits, `_`, `-` and one `.`.
fn check_name(name: &str, kind: &str) -> Result<(), String> {
    let (stem, ext) = name
        .rsplit_once('.')
        .ok_or_else(|| format!("{name:?}: no .{kind} extension"))?;
    if !ext.eq_ignore_ascii_case(kind) {
        return Err(format!("{name:?}: not a .{kind} file"));
    }
    if stem.is_empty()
        || !stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(format!(
            "{name:?}: use a plain file name (letters, digits, '_' and '-')"
        ));
    }
    if name.len() > MAX_NAME {
        return Err(format!("{name:?}: longer than {MAX_NAME} characters"));
    }
    Ok(())
}

/// The game's model-info stores (`CModelInfo::ms_*ModelInfoStore`, `ms_2dFXInfoStore`) by what a definition file
/// adds to them; the host raises each by what the mods add.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoreNeeds {
    /// `objs` without the damageable flag (`CAtomicModelInfo`).
    pub atomic: u32,
    /// `objs` with flag 0x1000 (`CDamageAtomicModelInfo`).
    pub damage_atomic: u32,
    /// `tobj` (`CTimeModelInfo`).
    pub time: u32,
    /// `weap` (`CWeaponModelInfo`).
    pub weapon: u32,
    /// `hier` and `anim` (`CClumpModelInfo`).
    pub clump: u32,
    /// `cars` (`CVehicleModelInfo`).
    pub vehicle: u32,
    /// `peds` (`CPedModelInfo`).
    pub ped: u32,
    /// `2dfx` (`C2dEffect`).
    pub effects_2d: u32,
    /// Collision models in collision files and archives (`CColModel`s of the collision-model pool).
    pub collision_models: u32,
    /// Texture dictionaries in archives and loose files (slots of the texture dictionary pool).
    pub texture_dictionaries: u32,
}

impl std::ops::Add for StoreNeeds {
    type Output = StoreNeeds;

    fn add(self, o: StoreNeeds) -> StoreNeeds {
        StoreNeeds {
            atomic: self.atomic + o.atomic,
            damage_atomic: self.damage_atomic + o.damage_atomic,
            time: self.time + o.time,
            weapon: self.weapon + o.weapon,
            clump: self.clump + o.clump,
            vehicle: self.vehicle + o.vehicle,
            ped: self.ped + o.ped,
            effects_2d: self.effects_2d + o.effects_2d,
            collision_models: self.collision_models + o.collision_models,
            texture_dictionaries: self.texture_dictionaries + o.texture_dictionaries,
        }
    }
}

impl std::ops::AddAssign for StoreNeeds {
    fn add_assign(&mut self, o: StoreNeeds) {
        *self = *self + o;
    }
}

impl fmt::Display for StoreNeeds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let all = [
            ("atomic", self.atomic),
            ("damage atomic", self.damage_atomic),
            ("time", self.time),
            ("weapon", self.weapon),
            ("clump", self.clump),
            ("vehicle", self.vehicle),
            ("ped", self.ped),
            ("2d effect", self.effects_2d),
            ("collision", self.collision_models),
            ("texture dictionary", self.texture_dictionaries),
        ];
        let parts: Vec<String> = all
            .iter()
            .filter(|p| p.1 > 0)
            .map(|(n, c)| format!("{c} {n}"))
            .collect();
        if parts.is_empty() {
            f.write_str("nothing")
        } else {
            f.write_str(&parts.join(", "))
        }
    }
}

/// A line as `CFileLoader::LoadLine` hands it on: leading blanks skipped, control characters and commas as
/// spaces.
fn load_line(raw: &[u8]) -> String {
    let s: String = raw
        .iter()
        .map(|&b| if b < b' ' || b == b',' { ' ' } else { b as char })
        .collect();
    s.trim_start().to_string()
}

/// What the definition file `text` adds to the model-info stores, read as `CFileLoader::LoadObjectTypes` reads it
/// (four-letter section tags, `end`, `#` comments); an error for a model id outside `0..model_count`.
pub fn ide_needs(text: &[u8], model_count: u32) -> Result<StoreNeeds, String> {
    let mut n = StoreNeeds::default();
    let mut section: Option<&str> = None;
    for (i, raw) in text.split(|&b| b == b'\n').enumerate() {
        let line = load_line(raw);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(sec) = section else {
            section = [
                "objs", "tobj", "weap", "hier", "anim", "cars", "peds", "path", "2dfx", "txdp",
            ]
            .into_iter()
            .find(|t| line.as_bytes().starts_with(t.as_bytes()));
            continue;
        };
        if line.starts_with("end") {
            section = None;
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let defines = !matches!(sec, "path" | "2dfx" | "txdp");
        if defines {
            let id = words.first().and_then(|w| w.parse::<i64>().ok());
            match id {
                Some(id) if (0..i64::from(model_count)).contains(&id) => {}
                _ => {
                    return Err(format!(
                        "line {}: model id {:?} outside 0..{model_count}",
                        i + 1,
                        words.first()
                    ));
                }
            }
        }
        match sec {
            "objs" => {
                let flags = words.last().and_then(|w| w.parse::<i64>().ok()).unwrap_or(0);
                if flags & 0x1000 == 0 {
                    n.atomic += 1;
                } else {
                    n.damage_atomic += 1;
                }
            }
            "tobj" => n.time += 1,
            "weap" => n.weapon += 1,
            "hier" | "anim" => n.clump += 1,
            "cars" => n.vehicle += 1,
            "peds" => n.ped += 1,
            "2dfx" => n.effects_2d += 1,
            _ => {}
        }
    }
    Ok(n)
}

/// The collision models of a collision file's bytes (`COLL`, `COL2`, `COL3`, `COL4` chunks, as
/// `CFileLoader::LoadCollisionFile` walks them).
pub fn collision_models(bytes: &[u8]) -> u32 {
    let mut n = 0;
    let mut p = 0usize;
    while let Some(h) = bytes.get(p..p + 8) {
        if !matches!(&h[..4], b"COLL" | b"COL2" | b"COL3" | b"COL4") {
            break;
        }
        n += 1;
        p += 8 + u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize;
    }
    n
}

/// The image archives the game's own level files register (`IMG` lines; `CFileLoader::LoadLevel` skips
/// `MODELS\GTA_INT.IMG`, which `CStreaming::InitImageList` already opened).
pub fn level_images(level_file: &[u8]) -> usize {
    level_file
        .split(|&b| b == b'\n')
        .map(load_line)
        .filter(|l| l.starts_with("IMG") && !l[3..].trim().starts_with("MODELS\\GTA_INT.IMG"))
        .count()
}

/// The lines a mod adds to the level file, its paths relative to the game folder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelLines {
    /// `IMG`, `COLFILE 0` and `IDE` lines: before the level file's first `IPL` line.
    pub before_placements: Vec<String>,
    /// `IPL` lines: after the level file's own.
    pub placements: Vec<String>,
}

impl LevelLines {
    /// The lines for `files` in the folder `dir` (relative to the game folder, `\`-separated).
    pub fn of(dir: &str, files: &DataFiles) -> LevelLines {
        let path = |f: &str| format!("{dir}\\{f}");
        let mut before = Vec::new();
        if !files.img.is_empty() {
            before.push(format!("IMG {}", path(ARCHIVES)));
        }
        before.extend(files.col.iter().map(|f| format!("COLFILE 0 {}", path(f))));
        before.extend(files.ide.iter().map(|f| format!("IDE {}", path(f))));
        LevelLines {
            before_placements: before,
            placements: files.ipl.iter().map(|f| format!("IPL {}", path(f))).collect(),
        }
    }
}

/// The level file `original` with the mods' lines (in load order): theirs before its first `IPL` line, their `IPL`
/// lines before its `EXIT` line or at its end. Line endings follow the file's.
pub fn merge_level_file(original: &[u8], mods: &[LevelLines]) -> Vec<u8> {
    let crlf = original.windows(2).any(|w| w == b"\r\n");
    let eol: &[u8] = if crlf { b"\r\n" } else { b"\n" };
    let mut lines: Vec<&[u8]> = original.split(|&b| b == b'\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let strip = |l: &[u8]| -> Vec<u8> { l.strip_suffix(b"\r").unwrap_or(l).to_vec() };
    let first_ipl = lines.iter().position(|l| load_line(l).starts_with("IPL"));
    let exit = lines.iter().position(|l| load_line(l).starts_with("EXIT"));
    let end = exit.unwrap_or(lines.len());
    let first_ipl = first_ipl.filter(|&p| p < end).unwrap_or(end);
    let mut out: Vec<Vec<u8>> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if i == first_ipl {
            out.extend(
                mods.iter()
                    .flat_map(|m| &m.before_placements)
                    .map(|s| s.as_bytes().to_vec()),
            );
        }
        if i == end {
            out.extend(
                mods.iter()
                    .flat_map(|m| &m.placements)
                    .map(|s| s.as_bytes().to_vec()),
            );
        }
        out.push(strip(l));
    }
    if first_ipl == lines.len() {
        out.extend(
            mods.iter()
                .flat_map(|m| &m.before_placements)
                .map(|s| s.as_bytes().to_vec()),
        );
    }
    if end == lines.len() {
        out.extend(
            mods.iter()
                .flat_map(|m| &m.placements)
                .map(|s| s.as_bytes().to_vec()),
        );
    }
    let mut v = out.join(eol);
    v.extend_from_slice(eol);
    v
}

/// A mod's data as the host hands it to the game: its files and where they come from.
pub struct ModData {
    /// The mod's id.
    pub id: String,
    /// Its folder as the game sees it, relative to the game folder (set by [`plan`]).
    pub dir: String,
    /// Its `[data]` files.
    pub files: DataFiles,
    /// The files, readable by the game's file layer (only the declared ones).
    pub fs: Box<dyn GameFs>,
}

/// What the mods' data does to the game's start-up ([`plan`]).
pub struct Plan {
    /// The level file with the mods' lines.
    pub level_file: Vec<u8>,
    /// What the mods' definitions add to the model-info stores.
    pub needs: StoreNeeds,
    /// The mods whose data loads, in load order.
    pub mods: Vec<ModData>,
}

/// Decides which mods' data fits the game's limits, in load order: `level_files` are the game's level files
/// (`DATA\DEFAULT.DAT` and `level_file`, the one the mods' lines go into), a mod's folder is `root\<id>`
/// relative to the game folder. A mod whose data does not fit is left out and reported to `log`.
pub fn plan(
    mods: Vec<ModData>,
    level_files: &[&[u8]],
    level_file: &[u8],
    root: &str,
    log: &mut dyn FnMut(String),
) -> Plan {
    let used = GAME_IMAGES + level_files.iter().map(|f| level_images(f)).sum::<usize>();
    let mut free = MAX_IMAGES.saturating_sub(used);
    let mut needs = StoreNeeds::default();
    let mut lines = Vec::new();
    let mut accepted = Vec::new();
    for m in mods {
        let d = format!("{root}\\{}", m.id);
        let check = || -> Result<(StoreNeeds, Option<Joined>), String> {
            let images = usize::from(!m.files.img.is_empty());
            if images > 0 && d.len() + 1 + ARCHIVES.len() > MAX_IMAGE_PATH {
                return Err(format!(
                    "the image path {d}\\{ARCHIVES} is longer than the game's {MAX_IMAGE_PATH} characters"
                ));
            }
            if images > free {
                return Err(format!(
                    "the game has no room for another image archive (it opens {MAX_IMAGES})"
                ));
            }
            let joined = if images > 0 {
                Some(Joined::new(m.fs.as_ref(), &m.files.img)?)
            } else {
                None
            };
            let mut n = StoreNeeds {
                collision_models: joined.as_ref().map_or(0, |j| j.collision_models),
                texture_dictionaries: joined.as_ref().map_or(0, |j| j.texture_dictionaries)
                    + m.files.txd.len() as u32,
                ..Default::default()
            };
            for f in &m.files.col {
                n.collision_models += collision_models(&m.fs.read(f).map_err(|e| format!("{f}: {e}"))?);
            }
            for f in &m.files.ide {
                let text = m.fs.read(f).map_err(|e| format!("{f}: {e}"))?;
                n += ide_needs(&text, sa_access::addr::MODEL_INFO_COUNT).map_err(|e| format!("{f}: {e}"))?;
            }
            if n.effects_2d > 0 {
                // a model's effects are indices into this store (`CBaseModelInfo::Add2dEffect`): it stays in place
                return Err(format!(
                    "{} 2dfx entries: the game's 2d-effect store does not grow",
                    n.effects_2d
                ));
            }
            Ok((n, joined))
        };
        match check() {
            Ok((n, joined)) => {
                needs += n;
                lines.push(LevelLines::of(&d, &m.files));
                log(format!(
                    "mod {}: data from {d}: {} files, definitions add {n}",
                    m.id,
                    m.files.all().count()
                ));
                let fs: Box<dyn GameFs> = match joined {
                    Some(j) => {
                        free -= 1;
                        log(format!(
                            "mod {}: {} image archives as {d}\\{ARCHIVES}: {} entries",
                            m.id,
                            m.files.img.len(),
                            j.entries
                        ));
                        Box::new(WithArchives {
                            fs: m.fs,
                            joined: Arc::new(j),
                        })
                    }
                    None => m.fs,
                };
                accepted.push(ModData { fs, dir: d, ..m });
            }
            Err(e) => log(format!("mod {}: data not loaded: {e}", m.id)),
        }
    }
    Plan {
        level_file: merge_level_file(level_file, &lines),
        needs,
        mods: accepted,
    }
}

/// A mod's image archives (`VER2`) as one: a directory of every entry, then each archive's bytes from a sector
/// boundary, the entries' offsets moved by where their archive starts. Entries keep their archive's order, the
/// archives the manifest's.
pub struct Joined {
    /// The header and directory, padded to a sector.
    dir: Vec<u8>,
    /// Each archive: (file, start in bytes, length).
    parts: Vec<(String, u64, u64)>,
    /// Its length.
    len: u64,
    /// The number of entries.
    pub entries: usize,
    /// The collision models of its `.col` entries.
    pub collision_models: u32,
    /// Its `.txd` entries.
    pub texture_dictionaries: u32,
}

impl Joined {
    /// Reads the directories of `files` in `fs`.
    pub fn new(fs: &dyn GameFs, files: &[String]) -> Result<Joined, String> {
        let mut dirs = Vec::new();
        let mut collision_models = 0;
        let mut texture_dictionaries = 0;
        for f in files {
            let mut r = fs.open(f).map_err(|e| format!("{f}: {e}"))?;
            let mut head = [0u8; 8];
            r.read_at(0, &mut head).map_err(|e| format!("{f}: {e}"))?;
            if &head[..4] != b"VER2" {
                return Err(format!("{f}: not a GTA San Andreas image archive (VER2)"));
            }
            let n = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
            let mut d = vec![0u8; n * 32];
            r.read_at(8, &mut d).map_err(|e| format!("{f}: directory: {e}"))?;
            for e in d.as_chunks::<32>().0 {
                let name = String::from_utf8_lossy(&e[8..])
                    .trim_end_matches('\0')
                    .to_ascii_lowercase();
                texture_dictionaries += u32::from(name.ends_with(".txd"));
                if name.ends_with(".col") {
                    let pos = u64::from(u32::from_le_bytes([e[0], e[1], e[2], e[3]])) * SECTOR;
                    let mut c = vec![0u8; usize::from(u16::from_le_bytes([e[4], e[5]])) * SECTOR as usize];
                    r.read_at(pos, &mut c)
                        .map_err(|err| format!("{f}: {name}: {err}"))?;
                    collision_models += self::collision_models(&c);
                }
            }
            dirs.push((f.clone(), d, r.len()));
        }
        let entries: usize = dirs.iter().map(|d| d.1.len() / 32).sum();
        let dir_len = (8 + 32 * entries as u64).div_ceil(SECTOR) * SECTOR;
        let mut dir = Vec::with_capacity(dir_len as usize);
        dir.extend_from_slice(b"VER2");
        dir.extend_from_slice(&(entries as u32).to_le_bytes());
        let mut parts = Vec::new();
        let mut at = dir_len;
        for (f, d, len) in dirs {
            let shift = u32::try_from(at / SECTOR).map_err(|_| format!("{f}: the archives are too large"))?;
            for e in d.as_chunks::<32>().0 {
                let pos = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
                dir.extend_from_slice(
                    &pos.checked_add(shift)
                        .ok_or(format!("{f}: the archives are too large"))?
                        .to_le_bytes(),
                );
                dir.extend_from_slice(&e[4..]);
            }
            parts.push((f, at, len));
            at += len.div_ceil(SECTOR) * SECTOR;
        }
        dir.resize(dir_len as usize, 0);
        Ok(Joined {
            dir,
            parts,
            len: at,
            entries,
            collision_models,
            texture_dictionaries,
        })
    }
}

/// The reads of a [`Joined`] archive, each archive opened once.
struct JoinedRead {
    joined: Arc<Joined>,
    parts: Vec<Box<dyn RandomRead>>,
}

impl JoinedRead {
    /// The part holding byte `pos` and its start, or `None` in the directory or the padding after an archive.
    fn part(&self, pos: u64) -> Option<(usize, u64, u64)> {
        let i = self.joined.parts.iter().rposition(|p| p.1 <= pos)?;
        let (_, start, len) = self.joined.parts[i];
        (pos < start + len).then_some((i, start, len))
    }
}

impl RandomRead for JoinedRead {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        if offset + buf.len() as u64 > self.joined.len {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let mut done = 0;
        while done < buf.len() {
            let pos = offset + done as u64;
            let rest = &mut buf[done..];
            let n = if let Some(d) = self.joined.dir.get(pos as usize..).filter(|d| !d.is_empty()) {
                let n = d.len().min(rest.len());
                rest[..n].copy_from_slice(&d[..n]);
                n
            } else if let Some((i, start, len)) = self.part(pos) {
                let n = ((start + len - pos) as usize).min(rest.len());
                self.parts[i].read_at(pos - start, &mut rest[..n])?;
                n
            } else {
                let next = self
                    .joined
                    .parts
                    .iter()
                    .map(|p| p.1)
                    .find(|&s| s > pos)
                    .unwrap_or(self.joined.len);
                let n = ((next - pos) as usize).min(rest.len());
                rest[..n].fill(0);
                n
            };
            done += n;
        }
        Ok(())
    }
    fn len(&self) -> u64 {
        self.joined.len
    }
    fn ready(&mut self, offset: u64, len: u64) -> bool {
        match self.part(offset) {
            Some((i, start, plen)) => self.parts[i].ready(offset - start, len.min(start + plen - offset)),
            None => true,
        }
    }
}

/// A mod's files with its joined image archive ([`ARCHIVES`]).
struct WithArchives {
    fs: Box<dyn GameFs>,
    joined: Arc<Joined>,
}

impl WithArchives {
    fn is_archives(path: &str) -> bool {
        matches!(sa_platform::fs::components(path).as_slice(), [one] if one.eq_ignore_ascii_case(ARCHIVES))
    }
}

impl GameFs for WithArchives {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        if !Self::is_archives(path) {
            return self.fs.read(path);
        }
        let mut r = self.open(path)?;
        let mut v = vec![0u8; r.len() as usize];
        r.read_at(0, &mut v)?;
        Ok(v)
    }
    fn open(&self, path: &str) -> std::io::Result<Box<dyn RandomRead>> {
        if !Self::is_archives(path) {
            return self.fs.open(path);
        }
        let parts = self
            .joined
            .parts
            .iter()
            .map(|p| self.fs.open(&p.0))
            .collect::<std::io::Result<_>>()?;
        Ok(Box::new(JoinedRead {
            joined: self.joined.clone(),
            parts,
        }))
    }
    fn exists(&self, path: &str) -> bool {
        Self::is_archives(path) || self.fs.exists(path)
    }
    fn is_dir(&self, path: &str) -> bool {
        self.fs.is_dir(path)
    }
}

/// The declared files of a mod's folder, as a game file tree: `fs` (the folder) answers only for them, by their
/// name in any case.
pub struct Declared {
    names: Vec<String>,
    fs: Box<dyn GameFs>,
}

impl Declared {
    /// The files of `files` in `fs`.
    pub fn new(files: &DataFiles, fs: Box<dyn GameFs>) -> Declared {
        Declared {
            names: files.all().cloned().collect(),
            fs,
        }
    }

    fn name(&self, path: &str) -> std::io::Result<&str> {
        let c = sa_platform::fs::components(path);
        match c.as_slice() {
            [one] => self
                .names
                .iter()
                .find(|n| n.eq_ignore_ascii_case(one))
                .map(String::as_str),
            _ => None,
        }
        .ok_or_else(|| std::io::ErrorKind::NotFound.into())
    }
}

impl GameFs for Declared {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        self.fs.read(self.name(path)?)
    }
    fn open(&self, path: &str) -> std::io::Result<Box<dyn RandomRead>> {
        self.fs.open(self.name(path)?)
    }
    fn exists(&self, path: &str) -> bool {
        self.name(path).is_ok_and(|n| self.fs.exists(n))
    }
    fn is_dir(&self, path: &str) -> bool {
        sa_platform::fs::components(path).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(t: &str) -> Result<(DataFiles, Vec<String>), String> {
        let v: toml::Table = t.parse().unwrap();
        let mut w = Vec::new();
        DataFiles::parse(&v["data"], &mut w).map(|f| (f, w))
    }

    #[test]
    fn parses_and_checks_names() {
        let (f, w) =
            files("[data]\nimg = [\"SAMP.img\"]\nide = [\"SAMP.ide\", \"CUSTOM.ide\"]\nifp = []\n").unwrap();
        assert_eq!(f.img, ["SAMP.img"]);
        assert_eq!(f.ide, ["SAMP.ide", "CUSTOM.ide"]);
        assert_eq!(w.len(), 1, "{w:?}");
        for bad in [
            "img = [\"../x.img\"]",
            "img = [\"x.ide\"]",
            "img = [\"x y.img\"]",
            "img = [\".img\"]",
            "img = [\"a.b.img\"]",
            "img = [\"abcdefghijklmnopqrstuvwxyz.img\"]",
            "img = [\"a.img\", \"A.IMG\"]",
            "img = \"a.img\"",
        ] {
            assert!(files(&format!("[data]\n{bad}\n")).is_err(), "{bad}");
        }
    }

    #[test]
    fn counts_what_definitions_add() {
        let ide = b"# c\nobjs\n18631, a, b, 100, 0\n 18632, a, b, 1, 100, 4096\nend\ntobj\n18633, a, b, 100, 0, 6, 20\nend\n\
                    anim\n18634, a, b, c, 100, 0\nend\npeds\n290, a, b, CIVMALE, STAT, man, 1, 0, null, 1, 1, PED, VOICE, VOICE\nend\n\
                    2dfx\n18631, 0, 0, 0, 1\nend\n";
        let n = ide_needs(ide, 20000).unwrap();
        assert_eq!(
            n,
            StoreNeeds {
                atomic: 1,
                damage_atomic: 1,
                time: 1,
                clump: 1,
                ped: 1,
                effects_2d: 1,
                ..Default::default()
            }
        );
        assert!(ide_needs(b"objs\n20000, a, b, 100, 0\nend\n", 20000).is_err());
        assert_eq!(collision_models(b"COL3\x04\0\0\0abcdCOLL\0\0\0\0xx"), 2);
        assert!(ide_needs(b"objs\nx, a, b, 100, 0\nend\n", 20000).is_err());
        assert_eq!(ide_needs(b"", 20000).unwrap(), StoreNeeds::default());
    }

    #[test]
    fn counts_the_level_files_images() {
        let dat = b"IMG MODELS\\GTA_INT.IMG\r\nIMG DATA\\SCRIPT\\SCRIPT.IMG\r\n#IMG X.IMG\r\n  IMG MODELS\\CUTSCENE.IMG\r\n";
        assert_eq!(level_images(dat), 2);
    }

    #[test]
    fn merges_lines_around_the_placements() {
        let f = DataFiles {
            img: vec!["S.img".into()],
            ide: vec!["S.ide".into()],
            col: vec!["S.col".into()],
            ipl: vec!["S.ipl".into()],
            txd: vec!["t.txd".into()],
        };
        let l = LevelLines::of("mods\\s", &f);
        let dat = b"IDE DATA\\A.IDE\r\n\r\nIPL DATA\\MAP.ZON\r\nIPL DATA\\B.IPL\r\n";
        assert_eq!(
            String::from_utf8(merge_level_file(dat, std::slice::from_ref(&l))).unwrap(),
            "IDE DATA\\A.IDE\r\n\r\nIMG mods\\s\\archives.all.img\r\nCOLFILE 0 mods\\s\\S.col\r\nIDE mods\\s\\S.ide\r\nIPL DATA\\MAP.ZON\r\n\
             IPL DATA\\B.IPL\r\nIPL mods\\s\\S.ipl\r\n"
        );
        let dat = b"IDE A\nEXIT\nIPL B\n";
        assert_eq!(
            String::from_utf8(merge_level_file(dat, std::slice::from_ref(&l))).unwrap(),
            "IDE A\nIMG mods\\s\\archives.all.img\nCOLFILE 0 mods\\s\\S.col\nIDE mods\\s\\S.ide\nIPL mods\\s\\S.ipl\nEXIT\nIPL B\n"
        );
        assert_eq!(merge_level_file(b"IDE A\n", &[]), b"IDE A\n");
    }

    /// A `VER2` archive with `entries` (name, bytes), each in its own sectors after a one-sector directory.
    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut v = b"VER2".to_vec();
        v.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        let mut data = Vec::new();
        for (name, bytes) in entries {
            let pos = 1 + data.len() as u32 / 2048;
            let sectors = (bytes.len() as u64).div_ceil(SECTOR) as u16;
            v.extend_from_slice(&pos.to_le_bytes());
            v.extend_from_slice(&sectors.to_le_bytes());
            v.extend_from_slice(&0u16.to_le_bytes());
            let mut n = [0u8; 24];
            n[..name.len()].copy_from_slice(name.as_bytes());
            v.extend_from_slice(&n);
            data.extend_from_slice(bytes);
            data.resize(data.len().div_ceil(2048) * 2048, 0);
        }
        v.resize(2048, 0);
        v.extend_from_slice(&data);
        v
    }

    /// Entry `i` of the archive `a`: (name, bytes).
    fn entry(a: &mut dyn RandomRead, i: u64) -> (String, Vec<u8>) {
        let mut e = [0u8; 32];
        a.read_at(8 + 32 * i, &mut e).unwrap();
        let pos = u64::from(u32::from_le_bytes([e[0], e[1], e[2], e[3]]));
        let mut d = vec![0u8; 4];
        a.read_at(pos * SECTOR, &mut d).unwrap();
        (
            String::from_utf8_lossy(&e[8..])
                .trim_end_matches('\0')
                .to_string(),
            d,
        )
    }

    #[test]
    fn joins_a_mods_archives() {
        let mut mem = sa_platform::fs::MemFs::default();
        mem.insert("a.img", archive(&[("x.dff", b"xxxx"), ("y.txd", &[b'y'; 3000])]));
        mem.insert("b.img", archive(&[("z.dff", b"zzzz")]));
        mem.insert("c.img", b"VER1".to_vec());
        let names = ["a.img".to_string(), "b.img".to_string()];
        let j = Joined::new(&mem, &names).unwrap();
        assert_eq!(j.entries, 3);
        let fs = WithArchives {
            fs: Box::new(mem),
            joined: Arc::new(j),
        };
        let mut r = fs.open(ARCHIVES).unwrap();
        assert_eq!(r.len(), 2048 * 7);
        let all: Vec<(String, Vec<u8>)> = (0..3).map(|i| entry(r.as_mut(), i)).collect();
        assert_eq!(
            all,
            [
                ("x.dff".into(), b"xxxx".to_vec()),
                ("y.txd".into(), b"yyyy".to_vec()),
                ("z.dff".into(), b"zzzz".to_vec())
            ]
        );
        // a read across the end of one archive into the next
        let mut span = vec![1u8; 8];
        r.read_at(2048 * 5 - 4, &mut span).unwrap();
        assert_eq!(span, b"\0\0\0\0VER2");
        assert!(fs.exists("ARCHIVES.ALL.IMG") && fs.read(ARCHIVES).unwrap().len() == 2048 * 7);
        let only_c = WithArchives {
            fs: fs.fs,
            joined: fs.joined,
        };
        assert!(Joined::new(only_c.fs.as_ref(), &["c.img".to_string()]).is_err());
    }

    #[test]
    fn answers_only_for_declared_files() {
        let mut mem = sa_platform::fs::MemFs::default();
        mem.insert("SAMP.img", vec![1]);
        mem.insert("other.img", vec![2]);
        let f = DataFiles {
            img: vec!["SAMP.img".into()],
            ..Default::default()
        };
        let d = Declared::new(&f, Box::new(mem));
        assert_eq!(d.read("samp.IMG").unwrap(), [1]);
        assert!(d.exists("SAMP.img") && !d.exists("other.img") && !d.exists("x\\SAMP.img"));
        assert!(d.read("other.img").is_err());
    }
}
