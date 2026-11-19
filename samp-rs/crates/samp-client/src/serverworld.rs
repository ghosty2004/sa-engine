//! What the server places in the world for the local player: pickups, actors, the checkpoint and the race
//! checkpoint, gang zones and map icons (the world RPCs other than objects and 3D labels), on any game behind
//! `GameAdapter`. The DLL client (`entities.rs`, on the original executable) and the hosted client (`hosted.rs`, on
//! the engine's mod API) both keep these records here and reach the game only through the adapter.
//!
//! Spec: docs/re/subsystems/world-entities.md; the original client's measured behaviour is in
//! docs/phase3/trace-notes.md ("World entities"). samp.dll addresses: docs/re/addresses/entities.md.

use std::collections::BTreeMap;

use sa_samp_api::peds::{AnimRequest, Proofs};
use sa_samp_api::streaming::ModelKind;
use sa_samp_api::world::{BlipTarget, CheckpointKind, RadarArea};
use sa_samp_api::{BlipRef, CheckpointRef, GameAdapter, ModelId, PedRef, PickupRef, Vec3};
use samp_proto::rpcs::world::{self as rw, Incoming as W};

use crate::netclient::NetClient;
use crate::worldents::{self as world, GangZone};

/// One of the server's pickups.
#[derive(Clone, Copy, Debug)]
pub struct Pickup {
    pub model: i32,
    pub kind: i32,
    pub pos: Vec3,
    pub handle: PickupRef,
    /// Collision reports left to wait before this pickup is reported again (counted down by the poll).
    cooldown: u32,
}

/// One of the server's actors.
#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub skin: u32,
    pub ped: PedRef,
    pub invulnerable: bool,
}

/// The normal checkpoint (drawn and tested every frame while active).
#[derive(Clone, Copy, Debug, Default)]
pub struct Checkpoint {
    pub active: bool,
    pub pos: Vec3,
    pub size: f32,
    pub blip: Option<BlipRef>,
}

/// The race checkpoint.
#[derive(Clone, Copy, Debug, Default)]
pub struct RaceCheckpoint {
    pub active: bool,
    pub kind: u8,
    pub pos: Vec3,
    pub next: Vec3,
    pub size: f32,
    pub blip: Option<BlipRef>,
    pub handle: Option<CheckpointRef>,
}

/// A line the client prints for a world RPC (samp's debug chat line).
pub type Notice = String;

/// The server's pickups, actors, checkpoints, gang zones and map icons, keyed by wire id.
#[derive(Default)]
pub struct ServerWorld {
    pub pickups: BTreeMap<i32, Pickup>,
    pub actors: BTreeMap<u16, Actor>,
    pub cp: Checkpoint,
    pub rcp: RaceCheckpoint,
    pub icons: BTreeMap<u8, BlipRef>,
    pub zones: BTreeMap<u16, GangZone>,
    frame: u64,
    /// RPC 131 sent so far.
    pub sent_131: u32,
}

/// The colour a blip gets for samp's colour value: the DLL patches `CRadar::GetRadarTraceColour` and passes samp's
/// values through, other hosts resolve them first (`worldents::radar_trace_colour`).
pub type BlipColour<'a> = &'a dyn Fn(u32) -> u32;

fn v3(p: samp_proto::types::Vec3) -> Vec3 {
    Vec3::new(p.x, p.y, p.z)
}

/// SA:MP text as a string, byte for byte (the names are ASCII).
fn text(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

/// A gang zone as the game's radar area: the wire colours are memory-order RGBA (`worldents::crgba_bytes`).
pub fn radar_area(z: &GangZone) -> RadarArea {
    let flash = (z.flash != z.colour).then_some((z.flash.swap_bytes(), world::GANG_ZONE_FLASH_MS));
    RadarArea { min: (z.rect[0], z.rect[3]), max: (z.rect[2], z.rect[1]), colour: z.colour.swap_bytes(), flash }
}

/// The model loaded like samp's creators: requested and loaded now, then polled (the streaming may finish it later)
/// for up to 10 s.
fn load_model(game: &mut dyn GameAdapter, model: i32) -> bool {
    let m = ModelId(model);
    if game.model_loaded(m) {
        return true;
    }
    let _ = game.request_model(m);
    let _ = game.load_all_now();
    let t0 = samp_proto::time::Instant::now();
    while !game.model_loaded(m) {
        if t0.elapsed() > std::time::Duration::from_secs(10) {
            return false;
        }
        std::hint::spin_loop();
    }
    true
}

impl ServerWorld {
    /// A world RPC; returns the line the client prints, if any.
    pub fn on_rpc(&mut self, game: &mut dyn GameAdapter, r: &W, colour: BlipColour<'_>) -> Option<Notice> {
        match r {
            W::PlayerCreatePickup(p) => self.create_pickup(game, p),
            W::PlayerDestroyPickup(p) => self.destroy_pickup(game, p.pickup_id),
            W::ShowActorForPlayer(a) => return self.show_actor(game, a),
            W::HideActorForPlayer(h) => self.delete_actor(game, h.actor_id),
            W::ApplyActorAnimationForPlayer(a) => self.apply_actor_animation(game, a),
            W::ClearActorAnimationsForPlayer(c) => {
                if let Some(a) = self.actors.get(&c.actor_id) {
                    let _ = game.flush_tasks(a.ped);
                }
            }
            W::SetActorFacingAngleForPlayer(f) => {
                if let Some(a) = self.actors.get(&f.actor_id) {
                    let _ = game.set_ped_heading(a.ped, world::samp_actor_heading(f.angle), true);
                }
            }
            W::SetActorPosForPlayer(p) => {
                if let Some(a) = self.actors.get(&p.actor_id) {
                    let _ = game.set_ped_position(a.ped, v3(p.position));
                }
            }
            W::SetActorHealthForPlayer(h) => {
                if let Some(a) = self.actors.get(&h.actor_id) {
                    set_actor_health(game, a.ped, h.health);
                }
            }
            W::SetCheckpoint(c) => {
                self.cp.pos = v3(c.position);
                self.cp.size = c.size;
                if let Some(b) = self.cp.blip {
                    let _ = game.remove_blip(b);
                    self.cp.blip = add_checkpoint_blip(game, self.cp.pos, colour);
                }
                self.cp.active = true;
            }
            W::DisableCheckpoint(_) => self.cp.active = false,
            W::SetRaceCheckpoint(c) => self.set_race_checkpoint(game, c, colour),
            W::DisableRaceCheckpoint(_) => self.rcp.active = false,
            // the client does not bounds-check the wire id; this one does
            W::ShowGangZone(z) if z.zone_id < world::MAX_GANG_ZONES => {
                let zone = GangZone::new(z.min.x, z.min.y, z.max.x, z.max.y, z.colour);
                self.zones.insert(z.zone_id, zone);
                let _ = game.show_radar_area(z.zone_id, &radar_area(&zone));
            }
            W::HideGangZone(z) => {
                if self.zones.remove(&z.zone_id).is_some() {
                    let _ = game.hide_radar_area(z.zone_id);
                }
            }
            W::FlashGangZone(z) => self.change_zone(game, z.zone_id, |r| r.flash = z.colour),
            W::StopFlashGangZone(z) => self.change_zone(game, z.zone_id, |r| r.flash = r.colour),
            W::SetPlayerMapIcon(m) => self.set_map_icon(game, m, colour),
            // the stored blip is removed even when there is none (handle 0), as samp does
            W::RemovePlayerMapIcon(m) if m.icon_id < world::MAX_MAP_ICONS => {
                let b = self.icons.remove(&m.icon_id).unwrap_or(BlipRef(0));
                let _ = game.remove_blip(b);
            }
            _ => {}
        }
        None
    }

    fn change_zone(&mut self, game: &mut dyn GameAdapter, id: u16, f: impl FnOnce(&mut GangZone)) {
        if let Some(z) = self.zones.get_mut(&id) {
            f(z);
            let _ = game.show_radar_area(id, &radar_area(z));
        }
    }

    /// RPC 95: the model loaded (an unknown model shows as "?"), then `CREATE_PICKUP`. Mirrors samp.dll 0x10013190.
    fn create_pickup(&mut self, game: &mut dyn GameAdapter, p: &rw::PlayerCreatePickup) {
        if !(0..world::MAX_PICKUPS).contains(&p.pickup_id) || self.pickups.len() >= world::MAX_PICKUPS as usize {
            return;
        }
        if let Some(old) = self.pickups.remove(&p.pickup_id) {
            let _ = game.remove_pickup(old.handle);
        }
        let mut model = p.model;
        if game.model_kind(ModelId(model)) == ModelKind::None {
            model = world::MODEL_UNKNOWN;
        }
        if (0..=world::MAX_MODEL).contains(&model) {
            load_model(game, model);
        }
        let pos = v3(p.position);
        if let Ok(handle) = game.create_pickup(ModelId(model), p.pickup_type, pos) {
            self.pickups.insert(p.pickup_id, Pickup { model, kind: p.pickup_type, pos, handle, cooldown: 0 });
        }
    }

    fn destroy_pickup(&mut self, game: &mut dyn GameAdapter, id: i32) {
        if let Some(old) = self.pickups.remove(&id) {
            let _ = game.remove_pickup(old.handle);
        }
    }

    /// The server's pickup whose game handle has `index` in its low word (the game's pickup array index).
    pub fn pickup_at_index(&self, index: u32) -> Option<i32> {
        self.pickups.iter().find(|(_, p)| p.handle.0 & 0xFFFF == index).map(|(&id, _)| id)
    }

    /// The local player touched pickup `id` (samp's collision path): true when it is to be reported now (the
    /// pickup is live and its cooldown has run out), which restarts the cooldown. Mirrors samp.dll 0x100A3F80.
    pub fn touch_pickup(&mut self, id: i32) -> bool {
        match self.pickups.get_mut(&id) {
            Some(p) if p.cooldown == 0 => {
                p.cooldown = world::PICKUP_COOLDOWN;
                true
            }
            _ => false,
        }
    }

    /// RPC 171. Mirrors samp.dll 0x10001900 and 0x1009C310.
    fn show_actor(&mut self, game: &mut dyn GameAdapter, a: &rw::ShowActorForPlayer) -> Option<Notice> {
        if a.actor_id >= world::MAX_ACTORS || game.model_kind(ModelId(a.skin as i32)) == ModelKind::None {
            return None;
        }
        let mut notice = None;
        if self.actors.contains_key(&a.actor_id) {
            notice = Some(format!("Warning: actor {} was not deleted", a.actor_id));
            self.delete_actor(game, a.actor_id);
        }
        if !load_model(game, a.skin as i32) {
            return notice;
        }
        let p = v3(a.position);
        let Ok(ped) = game.create_actor(ModelId(a.skin as i32), Vec3::new(p.x, p.y, p.z - 1.0), a.angle) else {
            return notice;
        };
        let _ = game.set_suffers_critical_hits(ped, false);
        let _ = game.set_decision_maker(ped, world::ACTOR_DECISION_MAKER);
        set_actor_health(game, ped, a.health);
        let [bullet, fire, explosion, collision, melee] = world::actor_proofs(a.invulnerable != 0).map(|v| v != 0);
        let _ = game.set_proofs(ped, Proofs { bullet, fire, explosion, collision, melee });
        self.actors.insert(a.actor_id, Actor { skin: a.skin, ped, invulnerable: a.invulnerable != 0 });
        notice
    }

    /// RPC 173: refused for the "SEX" library. Mirrors samp.dll 0x1009C4C0.
    fn apply_actor_animation(&mut self, game: &mut dyn GameAdapter, a: &rw::ApplyActorAnimationForPlayer) {
        let Some(ac) = self.actors.get(&a.actor_id).copied() else { return };
        if world::actor_anim_refused(&a.library) {
            return;
        }
        let (block, name) = (text(&a.library), text(&a.name));
        let anim = AnimRequest {
            name: &name,
            block: &block,
            delta: a.delta,
            looped: a.loop_anim,
            lock_x: a.lock_x,
            lock_y: a.lock_y,
            freeze: a.freeze,
            time_ms: a.time as i32,
        };
        let _ = game.play_anim(ac.ped, &anim);
    }

    fn delete_actor(&mut self, game: &mut dyn GameAdapter, id: u16) {
        if let Some(a) = self.actors.remove(&id) {
            let _ = game.destroy_ped(a.ped);
        }
    }

    /// RPC 38: the old race checkpoint deleted, a new one created. Mirrors samp.dll 0x100A2140.
    fn set_race_checkpoint(&mut self, game: &mut dyn GameAdapter, c: &rw::SetRaceCheckpoint, colour: BlipColour<'_>) {
        let r = &mut self.rcp;
        r.kind = c.checkpoint_type;
        r.pos = v3(c.position);
        r.next = v3(c.next_position);
        r.size = c.size;
        if let Some(b) = r.blip {
            let _ = game.remove_blip(b);
            r.blip = add_checkpoint_blip(game, r.pos, colour);
        }
        if let Some(h) = r.handle.take() {
            let _ = game.delete_checkpoint(h);
        }
        let kind = CheckpointKind::Race { kind: r.kind, next: r.next };
        r.handle = game.create_checkpoint(kind, r.pos, r.size).ok();
        r.active = true;
    }

    /// RPC 56. Mirrors samp.dll 0x1000A300 and 0x100A0DD0.
    fn set_map_icon(&mut self, game: &mut dyn GameAdapter, m: &rw::SetPlayerMapIcon, colour: BlipColour<'_>) {
        if m.icon_id >= world::MAX_MAP_ICONS {
            return;
        }
        if let Some(b) = self.icons.remove(&m.icon_id) {
            let _ = game.remove_blip(b);
        }
        let (short_range, contact_point) = world::map_icon_style(m.style);
        let target = BlipTarget::Coord { pos: v3(m.position), sprite: m.icon_type, short_range, contact_point };
        let Ok(b) = game.add_blip(target) else { return };
        if m.icon_type == 0 {
            let _ = game.set_blip_colour(b, colour(m.colour));
            let _ = game.set_blip_scale(b, world::map_icon_scale(m.colour));
        }
        self.icons.insert(m.icon_id, b);
    }

    /// Per frame, from the netgame tick: the checkpoint (its marker and its blip), the race checkpoint's blip, then
    /// the pickup poll every 7 frames (type-14 pickups through `HAS_PICKUP_BEEN_COLLECTED`, the others' collision
    /// cooldowns). Mirrors samp.dll 0x100A1F60 and 0x10013520.
    pub fn tick(&mut self, game: &mut dyn GameAdapter, net: &mut NetClient, colour: BlipColour<'_>) {
        self.frame += 1;
        if self.cp.active {
            if let Some(me) = game.local_player() {
                let s = self.cp.size;
                let _ = game.locate_ped(me, self.cp.pos, Vec3::new(s, s, s), true);
            }
            if self.cp.blip.is_none() {
                self.cp.blip = add_checkpoint_blip(game, self.cp.pos, colour);
            }
        } else if let Some(b) = self.cp.blip.take() {
            let _ = game.remove_blip(b);
        }
        if self.rcp.active {
            if self.rcp.blip.is_none() {
                self.rcp.blip = add_checkpoint_blip(game, self.rcp.pos, colour);
            }
        } else {
            if let Some(b) = self.rcp.blip.take() {
                let _ = game.remove_blip(b);
            }
            if let Some(h) = self.rcp.handle.take() {
                let _ = game.delete_checkpoint(h);
            }
        }
        if self.frame.is_multiple_of(world::PICKUP_POLL_FRAMES) {
            for (&id, p) in &mut self.pickups {
                if p.kind == world::PICKUP_TYPE_POLLED {
                    if game.pickup_collected(p.handle).unwrap_or(false) {
                        net.send_rpc(
                            &rw::OnPlayerPickUpPickup { pickup_id: id },
                            rw::OnPlayerPickUpPickup::POLL_SEND_PARAMS.reliability,
                        );
                        self.sent_131 += 1;
                    }
                } else {
                    p.cooldown = p.cooldown.saturating_sub(1);
                }
            }
        }
    }

    /// Reports pickup `id` the local player touched (RPC 131 over the collision path) when its cooldown allows.
    pub fn report_touch(&mut self, net: &mut NetClient, id: i32) {
        if self.touch_pickup(id) {
            net.send_rpc(&rw::OnPlayerPickUpPickup { pickup_id: id }, <rw::OnPlayerPickUpPickup as samp_proto::rpcs::OutgoingRpc>::RELIABILITY);
            self.sent_131 += 1;
        }
    }

    /// The collision path on hosts without samp's hook in `CPickup::Update`: every pickup the local player touches
    /// now, in id order.
    pub fn touched_pickups(&self, game: &dyn GameAdapter) -> Vec<i32> {
        self.pickups.iter().filter(|(_, p)| game.pickup_touched(p.handle).unwrap_or(false)).map(|(&id, _)| id).collect()
    }

    /// The pickup pool's deletion: `REMOVE_PICKUP` for every slot in id order. Mirrors samp.dll 0x10013410.
    pub fn clear_pickups(&mut self, game: &mut dyn GameAdapter) {
        for (_, p) in std::mem::take(&mut self.pickups) {
            let _ = game.remove_pickup(p.handle);
        }
    }

    /// The actor pool's deletion: every actor in id order. Mirrors samp.dll 0x100018E0.
    pub fn clear_actors(&mut self, game: &mut dyn GameAdapter) {
        for (_, a) in std::mem::take(&mut self.actors) {
            let _ = game.destroy_ped(a.ped);
        }
    }

    /// Everything gone (the server restarts or the connection was lost): pickups, actors, map icons and gang zones
    /// deleted; both checkpoints off (their blips and the race marker go at the next tick).
    pub fn clear(&mut self, game: &mut dyn GameAdapter) {
        self.clear_pickups(game);
        self.clear_actors(game);
        for (_, b) in std::mem::take(&mut self.icons) {
            let _ = game.remove_blip(b);
        }
        self.cp.active = false;
        self.rcp.active = false;
        for (id, _) in std::mem::take(&mut self.zones) {
            let _ = game.hide_radar_area(id);
        }
    }
}

/// RPC 178 and actor creation: the ped's health; at or below 0 `EXPLODE_CHAR_HEAD`. Mirrors samp.dll 0x1009C630.
fn set_actor_health(game: &mut dyn GameAdapter, ped: PedRef, health: f32) {
    let _ = game.set_ped_health(ped, health);
    if health <= 0.0 {
        let _ = game.explode_head(ped);
    }
}

/// A checkpoint's radar blip: a short-range coordinate blip with sprite 0, the checkpoint colour, scale 3. Mirrors
/// samp.dll 0x100A1F60.
fn add_checkpoint_blip(game: &mut dyn GameAdapter, pos: Vec3, colour: BlipColour<'_>) -> Option<BlipRef> {
    let target = BlipTarget::Coord { pos, sprite: 0, short_range: true, contact_point: false };
    let b = game.add_blip(target).ok()?;
    let _ = game.set_blip_colour(b, colour(world::BLIP_COLOUR_CHECKPOINT as u32));
    let _ = game.set_blip_scale(b, 3);
    Some(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gang_zones_become_radar_areas() {
        let mut z = GangZone::new(-10.0, -20.0, 10.0, 20.0, 0x8800_00FF);
        let a = radar_area(&z);
        assert_eq!((a.min, a.max, a.colour, a.flash), ((-10.0, -20.0), (10.0, 20.0), 0xFF00_0088, None));
        z.flash = 0x8800_FF00;
        assert_eq!(radar_area(&z).flash, Some((0x00FF_0088, world::GANG_ZONE_FLASH_MS)));
    }

    #[test]
    fn touched_pickups_wait_for_their_cooldown() {
        let mut w = ServerWorld::default();
        w.pickups.insert(3, Pickup { model: 1240, kind: 2, pos: Vec3::default(), handle: PickupRef(0x0005_0007), cooldown: 0 });
        assert_eq!(w.pickup_at_index(7), Some(3));
        assert!(w.touch_pickup(3));
        assert!(!w.touch_pickup(3));
        w.pickups.get_mut(&3).unwrap().cooldown = 1;
        w.pickups.get_mut(&3).unwrap().cooldown -= 1;
        assert!(w.touch_pickup(3));
        assert!(!w.touch_pickup(9));
    }
}
