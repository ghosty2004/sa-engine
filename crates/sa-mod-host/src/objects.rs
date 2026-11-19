//! The object, model and building-removal host functions (API 1.8).

use sa_mod_api::state::{MatrixInfo, ObjectInfo};
use sa_mod_api::{Call, attach, material, model, status};
use sa_types::{EntityRef, GameError, Matrix, ModelId, ModelKind, ObjectRef, PedRef, Vec3, VehicleRef};

use crate::api::{CallCtx, GameAccess, Mem, guarded, text, write_handles, write_record};

/// What a mod changed in the game's data that the host gives back when it disables the mod.
#[derive(Debug, Default)]
pub struct GameChanges {
    /// Models it requested (released).
    pub models: Vec<ModelId>,
    /// Draw distances it changed: (model, the original distance).
    pub draw_distances: Vec<(ModelId, f32)>,
}

impl GameChanges {
    /// Gives the game back what these changes took.
    pub fn undo(self, g: &mut GameAccess) {
        for m in self.models {
            let _ = g.release_model(m);
        }
        for (m, d) in self.draw_distances.into_iter().rev() {
            let _ = g.set_model_draw_distance(m, d);
        }
    }
}

/// Whether `call` is an object change that [`apply`] makes (plain values: deferred to the next frame boundary
/// inside a hook).
pub fn is_change(call: &Call) -> bool {
    matches!(
        call,
        Call::set_object_position { .. }
            | Call::set_object_rotation { .. }
            | Call::set_object_scale { .. }
            | Call::set_object_visible { .. }
            | Call::set_object_collision { .. }
            | Call::detach_object { .. }
    )
}

/// Applies an object change ([`is_change`]): its status.
pub fn apply(g: &mut GameAccess, call: Call, logs: &mut Vec<(i32, String)>) -> i32 {
    let obj = |h: i32| ObjectRef(h as u32);
    let finite = |v: &[f32]| v.iter().all(|x| x.is_finite());
    let r = match call {
        Call::set_object_position { handle, x, y, z } => {
            if !finite(&[x, y, z]) {
                return status::BAD_ARG;
            }
            guarded(logs, "set_object_position", || g.teleport_object(obj(handle), Vec3::new(x, y, z)))
        }
        Call::set_object_rotation { handle, x, y, z } => {
            if !finite(&[x, y, z]) {
                return status::BAD_ARG;
            }
            guarded(logs, "set_object_rotation", || g.set_object_rotation(obj(handle), Vec3::new(x, y, z)))
        }
        Call::set_object_scale { handle, scale } => {
            if !finite(&[scale]) {
                return status::BAD_ARG;
            }
            guarded(logs, "set_object_scale", || g.set_object_scale(obj(handle), scale))
        }
        Call::set_object_visible { handle, on } => {
            guarded(logs, "set_object_visible", || g.set_object_visible(obj(handle), on != 0))
        }
        Call::set_object_collision { handle, on } => {
            guarded(logs, "set_object_collision", || g.set_object_collision(obj(handle), on != 0))
        }
        Call::detach_object { handle } => guarded(logs, "detach_object", || g.detach_object(obj(handle))),
        _ => return status::BAD_ARG,
    };
    r.map_or_else(|s| s, |()| status::OK)
}

/// Reads `N` little-endian `f32` at `ptr`, all finite.
fn floats<const N: usize>(mem: &dyn Mem, ptr: i32) -> Option<[f32; N]> {
    let raw = mem.read(u32::try_from(ptr).ok()?, 4 * N as u32)?;
    let mut out = [0f32; N];
    for (o, w) in out.iter_mut().zip(raw.as_chunks::<4>().0) {
        *o = f32::from_le_bytes(*w);
    }
    out.iter().all(|v| v.is_finite()).then_some(out)
}

/// A model id with a model info, or `BAD_ARG`.
fn known_model(g: &GameAccess, m: i32) -> Result<ModelId, i32> {
    let id = ModelId(m);
    if (0..model::COUNT).contains(&m) && g.model_kind(id) != ModelKind::None { Ok(id) } else { Err(status::BAD_ARG) }
}

/// The object, model and building-removal calls that are no [`is_change`]: `None` for any other call.
pub fn calls(ctx: &mut CallCtx, mem: &mut dyn Mem, call: Call) -> Option<i32> {
    let reads = matches!(
        call,
        Call::object_info { .. }
            | Call::object_matrix { .. }
            | Call::object_list { .. }
            | Call::ped_bone_matrix { .. }
            | Call::model_info { .. }
    );
    let writes = matches!(
        call,
        Call::model_request { .. }
            | Call::model_release { .. }
            | Call::set_model_draw_distance { .. }
            | Call::set_object_matrix { .. }
            | Call::set_object_speed { .. }
            | Call::attach_object { .. }
            | Call::set_object_material { .. }
            | Call::remove_buildings { .. }
    );
    if !reads && !writes {
        return None;
    }
    if writes && ctx.in_hook() {
        return Some(status::WRONG_PHASE);
    }
    let Some(game) = ctx.game.clone() else { return Some(status::NOT_FOUND) };
    let mut g = game.0.borrow_mut();
    let obj = |h: i32| ObjectRef(h as u32);
    let done = |r: Result<(), i32>| r.map_or_else(|s| s, |()| status::OK);
    Some(match call {
        Call::object_info { handle, buf_ptr, buf_len } => match g.object_snapshot(obj(handle)) {
            Ok(s) => write_record(mem, buf_ptr, buf_len, &ObjectInfo::from(&s)),
            Err(_) => status::NOT_FOUND,
        },
        Call::object_matrix { handle, buf_ptr, buf_len } => match g.object_snapshot(obj(handle)) {
            Ok(s) => write_record(mem, buf_ptr, buf_len, &MatrixInfo::from(&s.matrix)),
            Err(_) => status::NOT_FOUND,
        },
        Call::object_list { buf_ptr, buf_len } => {
            write_handles(mem, buf_ptr, buf_len, g.objects().into_iter().map(|o| o.0))
        }
        Call::ped_bone_matrix { handle, node, buf_ptr, buf_len } => {
            let Some(node) = u8::try_from(node).ok().filter(|n| sa_access::objects::PED_NODES.contains(n)) else {
                return Some(status::BAD_ARG);
            };
            let ped = PedRef(handle as u32);
            if g.ped_addr(ped).is_err() {
                return Some(status::NOT_FOUND);
            }
            match guarded(&mut ctx.logs, "ped_bone_matrix", || g.ped_bone_matrix(ped, node)) {
                Ok(m) => write_record(mem, buf_ptr, buf_len, &MatrixInfo::from(&m)),
                Err(s) => s,
            }
        }
        Call::model_info { model: m } => {
            if !(0..model::COUNT).contains(&m) {
                return Some(status::BAD_ARG);
            }
            let id = ModelId(m);
            model::kind_number(g.model_kind(id)) | if g.model_loaded(id) { model::LOADED } else { 0 }
        }
        Call::model_request { model: m, now } => {
            let id = match known_model(&g, m) {
                Ok(id) => id,
                Err(s) => return Some(s),
            };
            let r = guarded(&mut ctx.logs, "model_request", || {
                g.request_model(id)?;
                if now != 0 { g.load_all_now() } else { Ok(()) }
            });
            if let Err(s) = r {
                return Some(s);
            }
            if !ctx.changes.models.contains(&id) {
                ctx.changes.models.push(id);
            }
            i32::from(g.model_loaded(id))
        }
        Call::model_release { model: m } => {
            let id = match known_model(&g, m) {
                Ok(id) => id,
                Err(s) => return Some(s),
            };
            ctx.changes.models.retain(|x| *x != id);
            done(guarded(&mut ctx.logs, "model_release", || g.release_model(id)))
        }
        Call::set_model_draw_distance { model: m, distance } => {
            let id = ModelId(m);
            if !(0..model::COUNT).contains(&m) || g.model_kind(id) != ModelKind::Atomic || distance.is_nan() || distance <= 0.0 {
                return Some(status::BAD_ARG);
            }
            match guarded(&mut ctx.logs, "set_model_draw_distance", || g.set_model_draw_distance(id, distance)) {
                Ok(old) => {
                    if !ctx.changes.draw_distances.iter().any(|(x, _)| *x == id) {
                        ctx.changes.draw_distances.push((id, old));
                    }
                    status::OK
                }
                Err(s) => s,
            }
        }
        Call::set_object_matrix { handle, matrix_ptr } => {
            let Some(w) = floats::<12>(mem, matrix_ptr) else { return Some(status::BAD_ARG) };
            let v = |i: usize| Vec3::new(w[i], w[i + 1], w[i + 2]);
            let m = Matrix { right: v(0), forward: v(3), up: v(6), pos: v(9) };
            done(guarded(&mut ctx.logs, "set_object_matrix", || g.set_object_matrix(obj(handle), &m)))
        }
        Call::set_object_speed { handle, speeds_ptr } => {
            let Some(w) = floats::<6>(mem, speeds_ptr) else { return Some(status::BAD_ARG) };
            let (mv, turn) = (Vec3::new(w[0], w[1], w[2]), Vec3::new(w[3], w[4], w[5]));
            done(guarded(&mut ctx.logs, "set_object_speed", || g.set_object_speed(obj(handle), mv, turn)))
        }
        Call::attach_object { handle, kind, target, offsets_ptr } => {
            let to = match kind {
                attach::PED => EntityRef::Ped(PedRef(target as u32)),
                attach::VEHICLE => EntityRef::Vehicle(VehicleRef(target as u32)),
                attach::OBJECT => EntityRef::Object(ObjectRef(target as u32)),
                _ => return Some(status::BAD_ARG),
            };
            let Some(w) = floats::<6>(mem, offsets_ptr) else { return Some(status::BAD_ARG) };
            let (offset, rot) = (Vec3::new(w[0], w[1], w[2]), Vec3::new(w[3], w[4], w[5]));
            done(guarded(&mut ctx.logs, "attach_object", || g.attach_object(obj(handle), to, offset, rot)))
        }
        Call::set_object_material { handle, slot, model: m, dict_ptr, dict_len, name_ptr, name_len, rgba } => {
            if !(0..material::SLOTS).contains(&slot) {
                return Some(status::BAD_ARG);
            }
            if g.object_addr(obj(handle)).is_err() {
                return Some(status::NOT_FOUND);
            }
            let texture = if m == -1 {
                0
            } else {
                let id = match known_model(&g, m) {
                    Ok(id) => id,
                    Err(s) => return Some(s),
                };
                let name = |p, l| text(mem, p, l).filter(|t| !t.is_empty() && t.len() <= material::MAX_NAME);
                let (Some(dict), Some(tex)) = (name(dict_ptr, dict_len), name(name_ptr, name_len)) else {
                    return Some(status::BAD_ARG);
                };
                let missing = |e| if e == GameError::NotLoaded { GameError::InvalidHandle } else { e };
                match guarded(&mut ctx.logs, "set_object_material", || {
                    g.model_texture(id, dict.as_bytes(), tex.as_bytes()).map_err(missing)
                }) {
                    Ok(t) => t,
                    Err(s) => return Some(s),
                }
            };
            done(guarded(&mut ctx.logs, "set_object_material", || {
                g.set_object_material(obj(handle), slot as usize, texture, rgba as u32)
            }))
        }
        Call::remove_buildings { model: m, x, y, z, radius } => {
            if !(-1..model::COUNT).contains(&m) || ![x, y, z, radius].iter().all(|v| v.is_finite()) || radius < 0.0 {
                return Some(status::BAD_ARG);
            }
            match guarded(&mut ctx.logs, "remove_buildings", || g.remove_buildings(m, Vec3::new(x, y, z), radius)) {
                Ok(n) => n.min(i32::MAX as u32) as i32,
                Err(s) => s,
            }
        }
        _ => return None,
    })
}
