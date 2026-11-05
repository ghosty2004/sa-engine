//! World and rendering tables: `timecyc.dat`, `popcycle.dat`, `water.dat`, `object.dat`,
//! `surfinfo.dat`, `surface.dat`, `surfaud.dat`, `procobj.dat`, `plants.dat`.
//!
//! Records keep the loader's variables after each line as [`Fields`](crate::data::misc::Fields).

pub mod object_dat;
pub mod plants;
pub mod popcycle;
pub mod procobj;
pub mod surface;
pub mod timecyc;
pub mod water;

use super::Parser;

/// Parser lookup for this group (see [`super::parser_for`]).
pub(crate) fn lookup(rel_path: &str) -> Option<Parser> {
    let p: Parser = match rel_path {
        "data/timecyc.dat" | "data/timecycp.dat" => |d| Ok(Box::new(timecyc::TimeCyc::parse(d))),
        "data/popcycle.dat" => |d| Ok(Box::new(popcycle::PopCycle::parse(d))),
        "data/water.dat" | "data/water1.dat" => |d| Ok(Box::new(water::Water::parse(d))),
        "data/object.dat" => |d| Ok(Box::new(object_dat::ObjectDat::parse(d))),
        "data/surface.dat" => |d| Ok(Box::new(surface::SurfaceDat::parse(d))),
        "data/surfinfo.dat" => |d| Ok(Box::new(surface::SurfInfo::parse(d))),
        "data/surfaud.dat" => |d| Ok(Box::new(surface::SurfAud::parse(d))),
        "data/procobj.dat" => |d| Ok(Box::new(procobj::ProcObj::parse(d))),
        "data/plants.dat" => |d| Ok(Box::new(plants::Plants::parse(d))),
        _ => return None,
    };
    Some(p)
}

/// Builds a counts vector, dropping zero entries.
fn counts(v: Vec<(String, u64)>) -> Vec<(String, u64)> {
    v.into_iter().filter(|(_, n)| *n > 0).collect()
}
