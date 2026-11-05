//! Other tables: `weapon.dat`, `melee.dat`, `ar_stats.dat`, `statdisp.dat`, `shopping.dat`,
//! `furnitur.dat`, `animgrp.dat`, `stream.ini`, `fonts.dat`, `gridref.dat`, `clothes.dat`,
//! train tracks, `roadblox.dat`, decision makers and `PedEvent.txt`.

pub mod animgrp;
pub mod clothes;
pub mod decision;
pub mod fonts;
pub mod furniture;
pub mod gridref;
pub mod melee;
pub mod roadblox;
pub mod shopping;
pub mod stats;
pub mod stream_ini;
pub mod tracks;
pub mod weapon;

use super::Parser;
pub(crate) use super::{Fields, Rec};

/// Parser lookup for this group (see [`super::parser_for`]).
pub(crate) fn lookup(rel_path: &str) -> Option<Parser> {
    let p: Parser = match rel_path {
        "data/weapon.dat" => |d| Ok(Box::new(weapon::WeaponDat::parse(d))),
        "data/melee.dat" => |d| Ok(Box::new(melee::MeleeDat::parse(d))),
        "data/ar_stats.dat" => |d| Ok(Box::new(stats::ArStats::parse(d))),
        "data/statdisp.dat" => |d| Ok(Box::new(stats::StatDisp::parse(d))),
        "data/shopping.dat" => |d| Ok(Box::new(shopping::Shopping::parse(d))),
        "data/furnitur.dat" => |d| Ok(Box::new(furniture::Furniture::parse(d))),
        "data/animgrp.dat" => |d| Ok(Box::new(animgrp::AnimGrp::parse(d))),
        "stream.ini" => |d| Ok(Box::new(stream_ini::StreamIni::parse(d))),
        "data/fonts.dat" => |d| Ok(Box::new(fonts::Fonts::parse(d))),
        "data/gridref.dat" => |d| Ok(Box::new(gridref::GridRef::parse(d))),
        "data/clothes.dat" => |d| Ok(Box::new(clothes::Clothes::parse(d))),
        "data/paths/tracks.dat"
        | "data/paths/tracks2.dat"
        | "data/paths/tracks3.dat"
        | "data/paths/tracks4.dat" => |d| Ok(Box::new(tracks::TrackFile::parse(d)?)),
        "data/paths/roadblox.dat" => |d| Ok(Box::new(roadblox::RoadBlox::parse(d)?)),
        "data/decision/pedevent.txt" => |d| Ok(Box::new(decision::PedEvents::parse(d))),
        // Only `data/decision/allowed/` is opened by the exe; the other folders hold an older
        // layout that the loader's format does not match.
        p if p.starts_with("data/decision/allowed/") && (p.ends_with(".ped") || p.ends_with(".grp")) => {
            |d| Ok(Box::new(decision::DecisionFile::parse(d)))
        }
        _ => return None,
    };
    Some(p)
}

/// `strtok(s, delims)` repeated until it returns NULL: the non-empty runs between delimiter
/// bytes (MSVC: the string ends at the first NUL).
pub(crate) fn strtok_all<'a>(s: &'a [u8], delims: &[u8]) -> Vec<&'a [u8]> {
    crate::bytes::c_str(s)
        .split(|b| delims.contains(b))
        .filter(|t| !t.is_empty())
        .collect()
}

/// Builds a counts vector, dropping zero entries.
pub(crate) fn counts(v: Vec<(String, u64)>) -> Vec<(String, u64)> {
    v.into_iter().filter(|(_, n)| *n > 0).collect()
}

/// `_stricmp(a, b) == 0` (ASCII case folding, the "C" locale).
pub(crate) fn stricmp_eq(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}
