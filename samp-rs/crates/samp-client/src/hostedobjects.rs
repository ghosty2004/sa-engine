//! The server's objects on a game that serves `sa_samp_api::GameAdapter` (the hosted client): the object pool of
//! RPC 44 / 45 / 46 / 47 / 99 / 122 / 75 / 84 / 169, the player-attached objects of RPC 113 and RemoveBuildingForPlayer
//! (RPC 43), driven by the rules the DLL client's `objects` module follows (`worldents`): the same create order,
//! the same MoveObject motion, the same attached-object placement. What the adapter does not serve (samp's own far
//! object pass, material text, the object editor) is in docs/ROADMAP.md's divergence ledger.

use std::collections::{BTreeMap, HashMap};

use samp_proto::rpcs::world::{self as rw, AttachedObjectData, ObjectMaterial};
use samp_proto::time::Instant;

use sa_samp_api::objects::{AttachTarget, MaterialOverride};
use sa_samp_api::{GameAdapter, Matrix, ModelId, ModelKind, ObjectRef, PedRef, Rgba, Vec3, VehicleRef};

use crate::footsync::Rotation;
use crate::worldents::{self as world, MATERIAL_SLOTS, Motion, MotionFrame, Transform};

/// What an object of RPC 44 is attached to, placed by the game once attached.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Attach {
    None,
    Vehicle { id: u16, offset: Vec3, rotation: Vec3 },
    Object { id: u16, offset: Vec3, rotation: Vec3 },
}

#[derive(Clone, Debug)]
struct Obj {
    model: i32,
    handle: ObjectRef,
    /// The rotation samp keeps for the object (SetObjectRotation, MoveObject's target rotation).
    rotation: [f32; 3],
    attach: Attach,
    motion: Option<Motion>,
    /// SetObjectNoCameraCollision: samp's own camera test (outside the map's sector grid) skips it; inside the
    /// grid the game's camera test runs as for any object, as with samp.dll.
    no_camera_collision: bool,
}

/// One player-attached object slot (RPC 113).
#[derive(Clone, Debug)]
struct Slot {
    data: AttachedObjectData,
    handle: ObjectRef,
}

/// Where the hosted client's players are, for the objects attached to them.
pub struct PlayerPeds<'a> {
    /// Our own player id.
    pub local_id: Option<u16>,
    /// The other players' stand-in peds, by player id.
    pub remote: &'a HashMap<u16, PedRef>,
}

impl PlayerPeds<'_> {
    fn ped(&self, id: u16, game: &dyn GameAdapter) -> Option<PedRef> {
        if Some(id) == self.local_id { game.local_player() } else { self.remote.get(&id).copied() }
    }
}

/// The hosted client's objects.
#[derive(Default)]
pub struct HostedObjects {
    pool: BTreeMap<u16, Obj>,
    attached: BTreeMap<(u16, u32), Slot>,
    removals: u32,
    epoch: Option<Instant>,
    last_pass: Option<Instant>,
}

fn v3(v: samp_proto::types::Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn arr(v: samp_proto::types::Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn transform(m: &Matrix) -> Transform {
    let a = |v: Vec3| [v.x, v.y, v.z];
    (Rotation { right: a(m.right), forward: a(m.forward), up: a(m.up) }, a(m.pos))
}

fn matrix((r, p): Transform) -> Matrix {
    let v = |a: [f32; 3]| Vec3::new(a[0], a[1], a[2]);
    Matrix { right: v(r.right), forward: v(r.forward), up: v(r.up), pos: v(p) }
}

/// A wire colour as the RenderWare material colour bytes samp writes (the value's little-endian bytes).
fn material_colour(c: u32) -> Rgba {
    Rgba::from_rgba_u32(c.swap_bytes())
}

/// The model samp creates for `model`: the "?" model when it has no model info.
fn known_model(game: &dyn GameAdapter, model: i32) -> i32 {
    if game.model_kind(ModelId(model)) == ModelKind::None { world::MODEL_UNKNOWN } else { model }
}

impl HostedObjects {
    /// Objects in the pool.
    pub fn count(&self) -> usize {
        self.pool.len()
    }

    fn now_ms(&mut self) -> u32 {
        self.epoch.get_or_insert_with(Instant::now).elapsed().as_millis() as u32
    }

    /// The object RPCs; returns a line for the client's log.
    pub fn on_rpc(&mut self, r: rw::Incoming, game: &mut dyn GameAdapter, peds: &PlayerPeds<'_>) -> Option<String> {
        use rw::Incoming as W;
        Some(match r {
            W::CreateObject(c) => self.create(c, game),
            W::SetObjectPosition(p) => {
                let o = self.pool.get(&p.object_id)?;
                let m = game.object_matrix(o.handle).ok()?;
                let r = game.set_object_matrix(o.handle, &Matrix { pos: v3(p.position), ..m });
                format!("SetObjectPosition {}: {r:?}", p.object_id)
            }
            W::SetObjectRotation(p) => {
                let o = self.pool.get_mut(&p.object_id)?;
                let pos = game.object_matrix(o.handle).ok()?.pos;
                o.rotation = arr(p.rotation);
                let r = game.set_object_transform(o.handle, pos, v3(p.rotation));
                format!("SetObjectRotation {}: {r:?}", p.object_id)
            }
            W::DestroyObject(d) => {
                let o = self.pool.remove(&d.object_id)?;
                format!("DestroyObject {}: {:?}", d.object_id, game.destroy_object(o.handle, false))
            }
            W::MoveObject(m) => {
                let now = self.now_ms();
                let o = self.pool.get_mut(&m.object_id)?;
                let current = transform(&game.object_matrix(o.handle).ok()?);
                let prev = o.motion.take();
                let (snap, motion) = world::begin_move(
                    prev.as_ref(),
                    current,
                    arr(m.target_position),
                    m.speed,
                    arr(m.target_rotation),
                    now,
                );
                if let Some(at) = snap {
                    let _ = game.set_object_speed(o.handle, Vec3::default(), Vec3::default());
                    let _ = game.set_object_matrix(o.handle, &matrix(at));
                    if let Some(r) = prev.and_then(|p| p.rotation) {
                        o.rotation = r;
                    }
                }
                o.motion = Some(motion);
                format!("MoveObject {} to {:?} at {}", m.object_id, arr(m.target_position), m.speed)
            }
            W::StopObject(s) => {
                let o = self.pool.get_mut(&s.object_id)?;
                o.motion = None;
                let r = game.set_object_speed(o.handle, Vec3::default(), Vec3::default());
                format!("StopObject {}: {r:?}", s.object_id)
            }
            W::AttachObjectToPlayer(a) => {
                let o = self.pool.get(&a.object_id)?;
                let ped = peds.ped(a.player_id, game)?;
                let r = game.attach_object(o.handle, AttachTarget::Ped(ped), v3(a.offset), v3(a.rotation));
                format!("AttachObjectToPlayer {} to {}: {r:?}", a.object_id, a.player_id)
            }
            W::SetPlayerObjectMaterial(m) => {
                let o = self.pool.get(&m.object_id)?.handle;
                format!("{}: {}", world::material_log_line(84, m.object_id, &m.material), apply_material(game, o, &m.material, false))
            }
            W::SetObjectNoCameraCollision(n) => {
                self.pool.get_mut(&n.object_id)?.no_camera_collision = true;
                format!("SetObjectNoCameraCollision {}", n.object_id)
            }
            W::SetPlayerAttachedObject(a) => self.attached_object(a, game, peds),
            _ => return None,
        })
    }

    /// RPC 44 CreateObject: a re-sent id replaces the old object; the exact position then the rotation; draw
    /// distance, attachment, materials and the camera flag. Mirrors samp.dll 0x100A9150.
    fn create(&mut self, c: rw::CreateObject, game: &mut dyn GameAdapter) -> String {
        if c.object_id >= world::MAX_OBJECTS {
            return format!("CreateObject {}: id out of range", c.object_id);
        }
        if let Some(old) = self.pool.remove(&c.object_id) {
            let _ = game.destroy_object(old.handle, false);
        }
        let model = known_model(game, c.model);
        let atomic = game.model_kind(ModelId(model)) == ModelKind::Atomic;
        if c.draw_distance != 0.0 && atomic {
            let _ = game.set_model_draw_distance(ModelId(model), c.draw_distance);
        }
        let handle = match game.create_object(ModelId(model), v3(c.position)) {
            Ok(h) => h,
            Err(e) => return format!("CreateObject {} model {model}: {e}", c.object_id),
        };
        let r = game.set_object_transform(handle, v3(c.position), v3(c.rotation));
        let attach = if c.attached_vehicle_id != rw::INVALID_ID {
            Attach::Vehicle { id: c.attached_vehicle_id, offset: v3(c.attach_offset), rotation: v3(c.attach_rotation) }
        } else if c.attached_object_id != rw::INVALID_ID {
            Attach::Object { id: c.attached_object_id, offset: v3(c.attach_offset), rotation: v3(c.attach_rotation) }
        } else {
            Attach::None
        };
        let mut line = format!(
            "CreateObject {} model {model} at ({:.2},{:.2},{:.2}) rot ({:.1},{:.1},{:.1}) as {:#x}: {r:?}",
            c.object_id, c.position.x, c.position.y, c.position.z, c.rotation.x, c.rotation.y, c.rotation.z, handle.0
        );
        for m in &c.materials {
            line.push_str(&format!("; {}", apply_material(game, handle, m, true)));
        }
        self.pool.insert(
            c.object_id,
            Obj {
                model,
                handle,
                rotation: arr(c.rotation),
                attach,
                motion: None,
                no_camera_collision: c.no_camera_collision != 0,
            },
        );
        line
    }

    /// RPC 113 SetPlayerAttachedObject: slots 0..=9, bones 1..=18; the slot's old object is deleted first. Mirrors
    /// samp.dll 0x10018F00.
    fn attached_object(&mut self, a: rw::SetPlayerAttachedObject, game: &mut dyn GameAdapter, peds: &PlayerPeds<'_>) -> String {
        if a.index > 9 {
            return format!("SetPlayerAttachedObject slot {}: out of range", a.index);
        }
        let key = (a.player_id, a.index);
        let Some(ped) = peds.ped(a.player_id, game) else {
            return format!("SetPlayerAttachedObject {}: no ped", a.player_id);
        };
        if let Some(old) = self.attached.remove(&key) {
            let _ = game.destroy_object(old.handle, false);
        }
        let Some(d) = a.data else { return format!("SetPlayerAttachedObject {} slot {} removed", a.player_id, a.index) };
        if !(1..=18).contains(&d.bone) {
            return format!("SetPlayerAttachedObject {}: bone {}", a.player_id, d.bone);
        }
        let model = known_model(game, d.model);
        if game.model_kind(ModelId(model)) == ModelKind::Atomic {
            let _ = game.set_model_draw_distance(ModelId(model), 200.0);
        }
        let pos = game.ped_snapshot(ped).map(|s| s.matrix.pos).unwrap_or_default();
        let handle = match game.create_object(ModelId(model), pos) {
            Ok(h) => h,
            Err(e) => return format!("SetPlayerAttachedObject {} model {model}: {e}", a.player_id),
        };
        let _ = game.set_object_transform(handle, pos, v3(d.rotation));
        for (slot, colour) in [(0u8, d.colour1), (1, d.colour2)] {
            if colour != 0 {
                let _ = game.set_object_material(handle, slot, Some(MaterialOverride::Colour(material_colour(colour))));
            }
        }
        let line = format!(
            "SetPlayerAttachedObject {} slot {} model {model} bone {} as {:#x}",
            a.player_id, a.index, d.bone, handle.0
        );
        self.attached.insert(key, Slot { data: d, handle });
        line
    }

    /// RPC 43 RemoveBuildingForPlayer.
    pub fn remove_building(&mut self, model: i32, centre: Vec3, radius: f32, game: &mut dyn GameAdapter) -> String {
        self.removals += 1;
        let r = game.remove_buildings(ModelId(model), centre, radius);
        format!("RemoveBuildingForPlayer model {model} at ({:.1},{:.1},{:.1}) radius {radius}: {r:?}", centre.x, centre.y, centre.z)
    }

    /// samp's object pass, every frame before the game's: attachments made once their target exists, moves
    /// advanced, attached slots placed on their bones.
    pub fn process(&mut self, game: &mut dyn GameAdapter, vehicles: &HashMap<u16, VehicleRef>, peds: &PlayerPeds<'_>) {
        let now = Instant::now();
        let dt = self.last_pass.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.0);
        self.last_pass = Some(now);
        let now_ms = self.now_ms();
        let time_step = game.time_step().clipped;
        let handles: BTreeMap<u16, ObjectRef> = self.pool.iter().map(|(k, o)| (*k, o.handle)).collect();
        for o in self.pool.values_mut() {
            let target = match o.attach {
                Attach::Vehicle { id, offset, rotation } => {
                    vehicles.get(&id).map(|v| (AttachTarget::Vehicle(*v), offset, rotation))
                }
                Attach::Object { id, offset, rotation } => {
                    handles.get(&id).map(|t| (AttachTarget::Object(*t), offset, rotation))
                }
                Attach::None => None,
            };
            if o.attach != Attach::None {
                if let Some((to, offset, rotation)) = target
                    && !game.object_attached(o.handle).unwrap_or(true)
                {
                    let _ = game.attach_object(o.handle, to, offset, rotation);
                }
                continue;
            }
            let Some(m) = o.motion else { continue };
            let Ok(current) = game.object_matrix(o.handle) else { continue };
            match world::motion_frame(&m, transform(&current), dt, now_ms, time_step) {
                MotionFrame::Arrive { at, rotation } => {
                    let _ = game.set_object_speed(o.handle, Vec3::default(), Vec3::default());
                    let _ = game.set_object_matrix(o.handle, &matrix(at));
                    if let Some(r) = rotation {
                        o.rotation = r;
                    }
                    o.motion = None;
                }
                MotionFrame::Move { move_speed, turn_speed, at } => {
                    // a move without a rotation leaves the turn speed as it was: 0 for a server object
                    let turn = turn_speed.map_or_else(Vec3::default, |t| Vec3::new(t[0], t[1], t[2]));
                    let _ = game.set_object_speed(o.handle, Vec3::new(move_speed[0], move_speed[1], move_speed[2]), turn);
                    let _ = game.set_object_matrix(o.handle, &matrix(at));
                }
                MotionFrame::Idle => {}
            }
        }
        for (&(pid, _), s) in &self.attached {
            let Some(ped) = peds.ped(pid, game) else { continue };
            let Ok(bone) = game.node_matrix(ped, s.data.bone as u8) else { continue };
            let at = world::attached_transform(transform(&bone), arr(s.data.offset), arr(s.data.rotation), arr(s.data.scale));
            let _ = game.set_object_matrix(s.handle, &matrix(at));
        }
    }

    /// Deletes every object (a new session).
    pub fn clear(&mut self, game: &mut dyn GameAdapter) {
        for (_, o) in std::mem::take(&mut self.pool) {
            let _ = game.destroy_object(o.handle, false);
        }
        for (_, s) in std::mem::take(&mut self.attached) {
            let _ = game.destroy_object(s.handle, false);
        }
    }

    /// One line per object, for the client's state report.
    pub fn status(&self, game: &dyn GameAdapter) -> String {
        let mut out = format!("objects={} attached={} removals={}", self.pool.len(), self.attached.len(), self.removals);
        for (id, o) in &self.pool {
            let p = game.object_matrix(o.handle).map(|m| m.pos).unwrap_or_default();
            out.push_str(&format!(
                " [{id} m{} ({:.2},{:.2},{:.2}) rot ({:.1},{:.1},{:.1}){}{}]",
                o.model,
                p.x,
                p.y,
                p.z,
                o.rotation[0],
                o.rotation[1],
                o.rotation[2],
                if o.motion.is_some() { " moving" } else { "" },
                if o.no_camera_collision { " nocam" } else { "" },
            ));
        }
        for ((pid, slot), s) in &self.attached {
            let m = game.object_matrix(s.handle).unwrap_or_default();
            out.push_str(&format!(
                " [player {pid} slot {slot} m{} bone {} ({:.3},{:.3},{:.3}) right ({:.2},{:.2},{:.2})]",
                s.data.model, s.data.bone, m.pos.x, m.pos.y, m.pos.z, m.right.x, m.right.y, m.right.z
            ));
        }
        out
    }
}

/// A material record (RPC 44 / 84) on the object: a texture of a model's dictionary and a colour. Material text and
/// the blank texture (model 0, SA:MP's own file) are not served: their colour still applies. Mirrors samp.dll
/// 0x100A8570.
fn apply_material(game: &mut dyn GameAdapter, o: ObjectRef, m: &ObjectMaterial, from_create: bool) -> String {
    match m {
        ObjectMaterial::Texture { slot, model, txd, texture, colour } => {
            if usize::from(*slot) >= MATERIAL_SLOTS || !world::material_name_ok(txd) || !world::material_name_ok(texture) {
                return format!("material slot {slot}: refused");
            }
            let model = world::material_model(*model, from_create);
            let colour = material_colour(*colour);
            let over = if model > 0 {
                MaterialOverride::Texture {
                    model: ModelId(model),
                    txd: String::from_utf8_lossy(txd).into_owned(),
                    texture: String::from_utf8_lossy(texture).into_owned(),
                    colour,
                }
            } else {
                MaterialOverride::Colour(colour)
            };
            format!("material slot {slot}: {:?}", game.set_object_material(o, *slot, Some(over)))
        }
        ObjectMaterial::Text { slot, .. } => format!("material text slot {slot}: not drawn on this host"),
        ObjectMaterial::Unknown { kind } => format!("material kind {kind}: unknown"),
    }
}
