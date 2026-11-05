//! Vehicle and population tables: `handling.cfg`, `carcols.dat`, `carmods.dat`, `ped.dat`,
//! `pedstats.dat`, `pedgrp.dat`, `cargrp.dat`.

pub mod carcols;
pub mod carmods;
pub mod groups;
pub mod handling;
pub mod ped_type;
pub mod pedstats;

use super::Parser;

/// `strtok` over `s` with the delimiter set `delims`: the non-empty runs between delimiters.
pub(crate) fn strtok<'a>(s: &'a [u8], delims: &[u8]) -> Vec<&'a [u8]> {
    s.split(|b| delims.contains(b))
        .filter(|t| !t.is_empty())
        .collect()
}

/// Parser lookup for this group (see [`super::parser_for`]).
pub(crate) fn lookup(rel_path: &str) -> Option<Parser> {
    Some(match rel_path {
        "data/handling.cfg" => |d| Ok(Box::new(handling::Handling::parse(d))),
        "data/carcols.dat" => |d| Ok(Box::new(carcols::CarCols::parse(d))),
        "data/carmods.dat" => |d| Ok(Box::new(carmods::CarMods::parse(d))),
        "data/ped.dat" => |d| Ok(Box::new(ped_type::PedTypes::parse(d))),
        "data/pedstats.dat" => |d| Ok(Box::new(pedstats::PedStats::parse(d))),
        "data/pedgrp.dat" => |d| Ok(Box::new(groups::Groups::parse(d, groups::Kind::Ped))),
        "data/cargrp.dat" => |d| Ok(Box::new(groups::Groups::parse(d, groups::Kind::Car))),
        _ => return None,
    })
}
