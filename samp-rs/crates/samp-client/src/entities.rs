//! The world RPCs on the original executable (Windows only): the records and the game-side work of pickups, actors,
//! checkpoints, gang zones and map icons are `serverworld` (shared with the hosted client, through the original
//! backend of the game adapter); this module adds samp's hooks (pickup collision, the actors' damage) and the 3D text
//! labels. Gang zones are drawn by the radar hooks in `worldhooks.rs` from the records the adapter publishes. Spec:
//! docs/re/subsystems/world-entities.md; the original's measured behaviour is in docs/phase3/trace-notes.md
//! ("World entities"). samp.dll addresses: docs/re/addresses/entities.md.

use std::collections::BTreeMap;
use std::sync::Mutex;

use sa_samp_api::peds::Peds as _;
use samp_proto::rpcs::world::{self as rw, Incoming as W};
use samp_proto::types::Vec3;

use crate::adapter::original::process;
use crate::netclient::NetClient;
use crate::players::Players;
use crate::script::{Arg, Runner};
use crate::serverworld::ServerWorld;
use crate::vehicles::Vehicles;
use crate::worldents as world;
use crate::worldhooks::{self as wh, Event, g, rd};

/// `IS_LINE_OF_SIGHT_CLEAR` (named via gta-reversed eScriptCommands.h; samp's descriptor has the parameter
/// types "ffffffiiiii").
const IS_LINE_OF_SIGHT_CLEAR: u16 = 0x06BD;
/// Camera position samp's label line-of-sight test starts from (a GTA camera object, vector at +0xC).
const LOS_CAMERA_POS: u32 = 0xB6F32C + 0xC;

fn vec(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// samp patches `CRadar::GetRadarTraceColour` (worldhooks), so blips keep samp's colour values.
const BLIP_COLOUR: &dyn Fn(u32) -> u32 = &|c| c;

#[derive(Clone, Debug)]
struct Label {
    colour: u32,
    pos: [f32; 3],
    draw_distance: f32,
    los: bool,
    player: u16,
    vehicle: u16,
    text: Vec<u8>,
}

pub use crate::ui::render::LabelDraw;

static LABEL_DRAWS: Mutex<Vec<LabelDraw>> = Mutex::new(Vec::new());

/// The labels of the current frame (UI Present pass).
pub fn label_draws() -> Vec<LabelDraw> {
    LABEL_DRAWS.try_lock().map(|g| g.clone()).unwrap_or_default()
}

/// The client's pickups, actors, checkpoints, gang zones and map icons ([`ServerWorld`]) and 3D labels, keyed by
/// wire id.
#[derive(Default)]
pub struct Entities {
    world: ServerWorld,
    labels: BTreeMap<u16, Label>,
    pub sent_177: u32,
    local_id: Option<u16>,
}

impl Entities {
    pub fn set_local_id(&mut self, id: u16) {
        self.local_id = Some(id);
    }

    pub fn on_rpc(&mut self, _runner: &mut Runner, r: W) {
        match r {
            W::PlayerShowTextLabel(l) => self.show_label(l),
            W::PlayerHideTextLabel(l) => {
                self.labels.remove(&l.label_id);
            }
            r => {
                if let Some(line) = self.world.on_rpc(&mut process(), &r, BLIP_COLOUR) {
                    crate::ui::print_debug(&line);
                }
            }
        }
    }

    fn show_label(&mut self, l: rw::PlayerShowTextLabel) {
        if l.label_id < world::MAX_LABELS {
            self.labels.insert(
                l.label_id,
                Label {
                    colour: l.colour,
                    pos: vec(l.position),
                    draw_distance: l.draw_distance,
                    los: l.test_los != 0,
                    player: l.attached_player_id,
                    vehicle: l.attached_vehicle_id,
                    // Stored as received (colour codes are kept for the painter's colour wrapper); a text shorter
                    // than 400 chars with `~k~` gets GTA's key-name expansion, kept if it fits in 399 chars.
                    text: samp_ui::text::expand_key_names_limit(
                        &l.text,
                        &crate::ui::GameExpander,
                        samp_ui::text::LABEL_KEY_EXPANSION_MAX_OUTPUT,
                    ),
                },
            );
        }
    }

    /// The streamed actors as (ped, actor id), published every tick to the combat hooks: their damage hook
    /// (combathooks.rs) sends RPC 177 for them, and the 204 packet carries them as the target actor.
    pub fn hook_actors(&self) -> Vec<(u32, u16)> {
        self.world.actors.iter().map(|(&id, a)| (a.ped.0, id)).collect()
    }

    /// Hook events (pickup collisions) → RPCs, in the order they happened.
    pub fn hook_events(&mut self, net: &mut NetClient) {
        let events = wh::with_shared(|s| std::mem::take(&mut s.events));
        for e in events {
            match e {
                Event::PickupTouched(index) => {
                    if let Some(id) = self.world.pickup_at_index(index) {
                        self.world.report_touch(net, id);
                    }
                }
            }
        }
    }

    /// Per frame, from the netgame tick: checkpoints and the pickup poll ([`ServerWorld::tick`]), then the labels.
    pub fn tick(&mut self, runner: &mut Runner, net: &mut NetClient, players: &Players, vehicles: &Vehicles) {
        self.world.tick(&mut process(), net, BLIP_COLOUR);
        self.labels_tick(runner, players, vehicles);
    }

    /// 3D label placement and screen projection for the Present pass. Mirrors samp.dll 0x10001350.
    fn labels_tick(&mut self, runner: &mut Runner, players: &Players, vehicles: &Vehicles) {
        let cam = rd_v3(g::CAMERA_POS);
        let mut out = Vec::new();
        for l in self.labels.values() {
            let pos = if l.player != rw::INVALID_ID {
                // A label attached to the local player is not drawn for that player.
                if Some(l.player) == self.local_id {
                    continue;
                }
                let ped = match players.get(l.player) {
                    Some(p) if l.player < 1004 => p.ped,
                    _ => continue,
                };
                if ped == 0 {
                    continue;
                }
                let Some(head) = head_position(ped) else { continue };
                let mut p = [head[0] + l.pos[0], head[1] + l.pos[1], head[2] + l.pos[2]];
                p[2] += 0.05 * world::dist3(p, cam);
                p
            } else if l.vehicle != rw::INVALID_ID {
                let Some(v) = vehicles.gta_ptr(l.vehicle) else { continue };
                let m: u32 = rd(v + g::ENTITY_MATRIX);
                if m == 0 {
                    continue;
                }
                let (r, f, u, o) = (rd_v3(m), rd_v3(m + 0x10), rd_v3(m + 0x20), rd_v3(m + 0x30));
                [0, 1, 2].map(|i| o[i] + r[i] * l.pos[0] + f[i] * l.pos[1] + u[i] * l.pos[2])
            } else {
                l.pos
            };
            if world::dist3(pos, cam) > l.draw_distance {
                continue;
            }
            if l.los {
                let c = rd_v3(LOS_CAMERA_POS);
                // One test per label and frame, as in the original.
                let args = [
                    Arg::Float(pos[0]),
                    Arg::Float(pos[1]),
                    Arg::Float(pos[2]),
                    Arg::Float(c[0]),
                    Arg::Float(c[1]),
                    Arg::Float(c[2]),
                    Arg::Int(1),
                    Arg::Int(0),
                    Arg::Int(0),
                    Arg::Int(1),
                    Arg::Int(0),
                ];
                let clear = runner.run(IS_LINE_OF_SIGHT_CLEAR, &args);
                if !clear {
                    continue;
                }
            }
            out.push(LabelDraw { pos, text: l.text.clone(), argb: world::label_argb(l.colour) });
        }
        if let Ok(mut g) = LABEL_DRAWS.lock() {
            *g = out;
        }
    }

    pub fn clear(&mut self, _runner: &mut Runner) {
        self.world.clear(&mut process());
        self.labels.clear();
        if let Ok(mut g) = LABEL_DRAWS.lock() {
            g.clear();
        }
    }

    /// The pickup pool's deletion ([`ServerWorld::clear_pickups`]).
    pub fn clear_pickups(&mut self, _runner: &mut Runner) {
        self.world.clear_pickups(&mut process());
    }

    /// The actor pool's deletion ([`ServerWorld::clear_actors`]).
    pub fn clear_actors(&mut self, _runner: &mut Runner) {
        self.world.clear_actors(&mut process());
    }

    pub fn header_fields(&self) -> String {
        let w = &self.world;
        format!(
            "pickups={} actors={} checkpoint={} race_checkpoint={} zones={} icons={} labels={} sent_131={} sent_177={}",
            w.pickups.len(),
            w.actors.len(),
            u8::from(w.cp.active),
            u8::from(w.rcp.active),
            w.zones.len(),
            w.icons.len(),
            self.labels.len(),
            w.sent_131,
            self.sent_177
        )
    }

    pub fn status_lines(&self, out: &mut Vec<String>) {
        let w = &self.world;
        let h = |v: Option<u32>| v.unwrap_or(0) as i32;
        for (id, p) in &w.pickups {
            out.push(format!(
                "pickup id={id} model={} type={} handle={} pos={:.3},{:.3},{:.3}",
                p.model, p.kind, p.handle.0 as i32, p.pos.x, p.pos.y, p.pos.z
            ));
        }
        for (id, a) in &w.actors {
            let ped = a.ped.0;
            let handle = process().ped_script_handle(a.ped).map_or(0, |s| s.0);
            let m: u32 = rd(ped + g::ENTITY_MATRIX);
            let p = if m != 0 { rd_v3(m + 0x30) } else { [0.0; 3] };
            let hp = rd::<f32>(ped + crate::game::off::PED_HEALTH);
            out.push(format!(
                "actor id={id} skin={} handle={handle} ped={ped:#x} pos={:.3},{:.3},{:.3} health={hp:.1} invulnerable={}",
                a.skin,
                p[0],
                p[1],
                p[2],
                u8::from(a.invulnerable)
            ));
        }
        if w.cp.active || w.cp.blip.is_some() {
            let c = &w.cp;
            out.push(format!(
                "checkpoint active={} pos={:.3},{:.3},{:.3} size={:.2} blip={}",
                u8::from(c.active),
                c.pos.x,
                c.pos.y,
                c.pos.z,
                c.size,
                h(c.blip.map(|b| b.0))
            ));
        }
        if w.rcp.active || w.rcp.handle.is_some() {
            let c = &w.rcp;
            out.push(format!(
                "race_checkpoint active={} type={} pos={:.3},{:.3},{:.3} next={:.3},{:.3},{:.3} size={:.2} blip={} handle={}",
                u8::from(c.active),
                c.kind,
                c.pos.x,
                c.pos.y,
                c.pos.z,
                c.next.x,
                c.next.y,
                c.next.z,
                c.size,
                h(c.blip.map(|b| b.0)),
                h(c.handle.map(|c| c.0))
            ));
        }
        for (id, z) in &w.zones {
            out.push(format!(
                "zone id={id} rect={:.1},{:.1},{:.1},{:.1} colour={:08x} flash={:08x}",
                z.rect[0], z.rect[1], z.rect[2], z.rect[3], z.colour, z.flash
            ));
        }
        for (id, b) in &w.icons {
            out.push(format!("icon id={id} blip={}", b.0 as i32));
        }
        for (id, l) in &self.labels {
            out.push(format!(
                "label id={id} colour={:08x} pos={:.3},{:.3},{:.3} dd={:.1} los={} player={} vehicle={} text={:?}",
                l.colour,
                l.pos[0],
                l.pos[1],
                l.pos[2],
                l.draw_distance,
                u8::from(l.los),
                l.player,
                l.vehicle,
                String::from_utf8_lossy(&l.text)
            ));
        }
        let drawn = label_draws().len();
        out.push(format!("labels_drawn count={drawn}"));
    }
}

fn rd_v3(a: u32) -> [f32; 3] {
    [rd(a), rd(a + 4), rd(a + 8)]
}

/// Head bone (8) position of a ped, from its animation hierarchy.
fn head_position(ped: u32) -> Option<[f32; 3]> {
    let frame_data: u32 = rd(ped + g::PED_BONES + 4 * 8);
    if frame_data == 0 {
        return None;
    }
    let node_id: i32 = rd(frame_data + 0x14);
    let clump: u32 = rd(ped + g::ENTITY_RW_OBJECT);
    if clump == 0 {
        return None;
    }
    // SAFETY: GetAnimHierarchyFromSkinClump / RpHAnimIDGetIndex (cdecl) on the ped's live clump.
    let hier = unsafe {
        let f: extern "C" fn(u32) -> u32 = std::mem::transmute(g::GET_ANIM_HIERARCHY_FROM_SKIN_CLUMP as usize);
        f(clump)
    };
    if hier == 0 {
        return None;
    }
    // SAFETY: RpHAnimIDGetIndex (cdecl) on the hierarchy just returned for the live clump.
    let idx = unsafe {
        let f: extern "C" fn(u32, i32) -> i32 = std::mem::transmute(g::RP_HANIM_ID_GET_INDEX as usize);
        f(hier, node_id)
    };
    let mats: u32 = rd(hier + 8);
    (idx >= 0 && mats != 0).then(|| rd_v3(mats + 0x40 * idx as u32 + 0x30))
}
