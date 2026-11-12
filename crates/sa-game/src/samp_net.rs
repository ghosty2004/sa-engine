//! `--samp` with the `samp` feature: a minimal SA:MP client on samp-rs's network core (`NetClient`, the sync
//! builders of `footsync`) that drives the local player through the native game adapter (docs/samp/native-plan.md
//! §3). It joins, picks the first class, spawns, applies the player RPCs a gamemode uses on its own player
//! (position, facing, health, armour, weapons, money, clock, weather, interior, control, camera) through
//! `sa_samp_api` writes and sends on-foot and stats sync from `ped_snapshot`.
//!
//! This stands in for samp-rs's full client until its game logic is linkable here (migration step 10); remote
//! players, vehicles, objects and the UI come with it. Everything runs on the game thread inside the frame
//! callback; RakNet's update thread is samp-rs's own.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use samp_client::footsync::{self, FootSender, KeyContext, LocalSample, Rotation};
use samp_client::netclient::{Event, NetClient, NetPacket, resolve};
use samp_proto::auth::AuthTable;
use samp_proto::raknet::Reliability;
use samp_proto::rpcs::{Incoming, player, remote, ui, vehicle};
use samp_proto::sync::player::StatsSync;

use sa_samp_api::camera::CameraCut;
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::ui::{ChatKind, ChatMessage, DialogSpec, DialogStyle, UiEvent};
use sa_samp_api::vehicles::VehicleSpawn;
use sa_samp_api::{AreaCode, GameAdapter, ModelId, Rgba, Vec3, VehicleRef, WeaponType};

use crate::samp::Target;

/// What the client did, for the run report (shared with `samp.rs`).
pub type Journal = Rc<RefCell<Vec<String>>>;

/// The native client's state.
pub struct NetHooks {
    net: NetClient,
    start: Instant,
    journal: Journal,
    connected: bool,
    on_foot_rate: u32,
    spawn: Option<player::SpawnInfo>,
    spawn_requested: bool,
    spawned: bool,
    sender: FootSender,
    last_stats: Option<Instant>,
    syncs_sent: u32,
    last_report: Option<Instant>,
    world_set_up: bool,
    /// Server vehicle id → the engine's vehicle (streamed in by RPC 164).
    vehicles: HashMap<u16, VehicleRef>,
    last_car_sync: Option<Instant>,
    car_rotation: Option<samp_proto::types::Quat>,
    /// Remote players' names (RPC 137), for player chat lines.
    names: HashMap<u16, String>,
}

/// The auth table (third-party data, never committed): `--samp-auth FILE` or `SAMP_AUTH_TABLE`.
fn auth_table(path: Option<&Path>) -> Result<AuthTable, String> {
    let p: PathBuf = match path {
        Some(p) => p.into(),
        None => std::env::var_os("SAMP_AUTH_TABLE")
            .map(PathBuf::from)
            .ok_or("samp: no auth table (--samp-auth FILE or SAMP_AUTH_TABLE)")?,
    };
    let text = std::fs::read_to_string(&p).map_err(|e| format!("samp: {}: {e}", p.display()))?;
    AuthTable::parse(&text).map_err(|e| format!("samp: {}: {e:?}", p.display()))
}

/// The client serial's seed: made once per install from randomness and kept in sa-game's configuration
/// (`$XDG_CONFIG_HOME/sa-engine/samp.cfg`, line `serial_seed=<40 hex digits>`); never derived from user or host
/// data (main-session decision 2026-11-12).
pub fn serial_seed() -> String {
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir)
        .join("sa-engine");
    let file = dir.join("samp.cfg");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    if let Some(seed) = parse_seed(&text) {
        return seed;
    }
    let seed = samp_client::netclient::pseudo_random_hex40();
    let _ = std::fs::create_dir_all(&dir);
    let mut out = text;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    let _ = writeln!(out, "serial_seed={seed}");
    if let Err(e) = std::fs::write(&file, out) {
        println!("samp: cannot keep the serial seed in {}: {e}", file.display());
    }
    seed
}

fn parse_seed(text: &str) -> Option<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("serial_seed="))
        .map(str::trim)
        .find(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_string)
}

impl NetHooks {
    /// Opens the session (the first request goes out with the first update).
    pub fn connect(target: &Target, auth: Option<&Path>, journal: Journal) -> Result<Self, String> {
        let addr = resolve(&target.host, target.port).map_err(|e| format!("samp: {}: {e}", target.host))?;
        let mut net =
            NetClient::new(addr, &target.name, auth_table(auth)?).map_err(|e| format!("samp: {e}"))?;
        net.set_serial_seed(&serial_seed());
        net.connect().map_err(|e| format!("samp: connect: {e}"))?;
        Ok(Self {
            net,
            start: Instant::now(),
            journal,
            connected: false,
            on_foot_rate: 30,
            spawn: None,
            spawn_requested: false,
            spawned: false,
            sender: FootSender::default(),
            last_stats: None,
            syncs_sent: 0,
            last_report: None,
            world_set_up: false,
            vehicles: HashMap::new(),
            last_car_sync: None,
            car_rotation: None,
            names: HashMap::new(),
        })
    }

    fn note(&self, line: String) {
        let line = format!("{:>6} ms  {line}", self.start.elapsed().as_millis());
        println!("samp: {line}");
        self.journal.borrow_mut().push(line);
    }

    fn tick(&mut self, game: &mut dyn GameAdapter) {
        let now = Instant::now();
        if !self.world_set_up {
            self.world_set_up = true;
            self.world_setup(game);
        }
        for e in self.net.tick(now) {
            self.on_event(e, game);
        }
        self.ui_events(game);
        if self.spawned {
            self.sync(game, now);
        }
        if self
            .last_report
            .is_none_or(|t| now.duration_since(t) >= Duration::from_secs(2))
        {
            self.last_report = Some(now);
            if let Some(me) = game.local_player()
                && let Ok(s) = game.ped_snapshot(me)
            {
                let p = s.matrix.pos;
                self.note(format!(
                    "state: phase={:?} spawned={} pos=({:.2},{:.2},{:.2}) health={:.0} armour={:.0} weapon={} money={} clock={:?} weather={:?} area={} syncs={} pad=({},{}) ped_flags={:#x}",
                    self.net.phase(),
                    self.spawned,
                    p.x,
                    p.y,
                    p.z,
                    s.health,
                    s.armour,
                    s.weapon.0,
                    game.player_money(),
                    game.clock(),
                    game.weather(),
                    game.area_raw(),
                    self.syncs_sent,
                    game.pad().0.left_x,
                    game.pad().0.left_y,
                    s.flags[0],
                ));
            }
        }
    }

    /// What samp's own main.scm does before the server takes over (samp-rs `scm.rs`): no ambient peds or traffic
    /// (densities 0), wanted level capped at 0, every road switched off, the player frozen until the spawn.
    fn world_setup(&mut self, game: &mut dyn GameAdapter) {
        report(self, "set_densities", game.set_densities(0.0, 0.0));
        report(self, "set_max_wanted_level", game.set_max_wanted_level(0));
        report(self, "switch_roads_off", game.switch_roads_off());
        report(self, "set_player_control", game.set_player_control(false));
        self.note("world set up like samp's main.scm (densities 0, wanted 0, roads off, control off)".into());
    }

    fn on_event(&mut self, e: Event, game: &mut dyn GameAdapter) {
        match e {
            Event::InitGame(g) => {
                self.connected = true;
                self.on_foot_rate = g.on_foot_rate.clamp(10, 1000);
                self.note(format!(
                    "InitGame: server {:?}, our id {:?}, hour {}, weather {}, gravity {}",
                    g.server_name,
                    self.net.player_id(),
                    g.world_time,
                    g.weather,
                    g.gravity
                ));
                report(self, "set_clock", game.set_clock(g.world_time, 0));
                report(self, "set_weather", game.set_weather(g.weather));
                report(self, "set_gravity", game.set_gravity(g.gravity));
                report(self, "set_zone_names", game.set_zone_names(false));
                self.net.send_rpc(
                    &player::PlayerRequestClass { class_index: 0 },
                    Reliability::Reliable,
                );
                self.note("requesting class 0".into());
            }
            Event::Packet(p) => {
                self.note(format!("packet {p:?}"));
                if matches!(
                    p,
                    NetPacket::ServerClosed | NetPacket::ConnectionLost | NetPacket::Banned
                ) {
                    self.spawned = false;
                }
            }
            Event::Rejected(r) => self.note(format!("connection rejected: {r}")),
            Event::Rpc(Incoming::Player(p)) => self.on_player_rpc(p, game),
            Event::Rpc(Incoming::Vehicle(v)) => self.on_vehicle_rpc(v, game),
            Event::Rpc(Incoming::Ui(u)) => self.on_ui_rpc(u, game),
            Event::Rpc(Incoming::Remote(remote::Incoming::PlayerJoin(j))) => {
                self.names.insert(j.player_id, text(&j.name));
            }
            Event::Rpc(other) => self.note(format!("rpc {} (not applied natively yet)", other_id(&other))),
            Event::Sync(_) => {}
        }
    }

    fn on_player_rpc(&mut self, rpc: player::Incoming, game: &mut dyn GameAdapter) {
        use player::Incoming as P;
        let Some(me) = game.local_player() else {
            self.note(format!("rpc {}: no local player yet", rpc.id()));
            return;
        };
        let v3 = |p: samp_proto::types::Vec3| Vec3::new(p.x, p.y, p.z);
        match rpc {
            P::PlayerRequestClassResponse(r) => {
                self.note(format!(
                    "class response: selectable={} skin={} pos=({:.1},{:.1},{:.1})",
                    r.selectable, r.spawn.skin, r.spawn.pos.x, r.spawn.pos.y, r.spawn.pos.z
                ));
                self.spawn = Some(r.spawn);
                if r.selectable != 0 && !self.spawn_requested {
                    self.spawn_requested = true;
                    self.net
                        .send_rpc(&player::PlayerRequestSpawn, Reliability::Reliable);
                    self.note("requesting spawn".into());
                }
            }
            P::SetSpawnInfo(s) => self.spawn = Some(s.spawn),
            P::PlayerRequestSpawnResponse(r) => {
                self.note(format!("spawn response {}", r.response));
                if r.response != 0 {
                    self.spawn_local(game);
                }
            }
            P::SetPlayerPosition(p) => {
                report(self, "set_ped_position", game.set_ped_position(me, v3(p.pos)));
                self.note(format!(
                    "SetPlayerPosition ({:.2},{:.2},{:.2})",
                    p.pos.x, p.pos.y, p.pos.z
                ));
            }
            P::SetPlayerFacingAngle(a) => {
                report(self, "set_ped_heading", game.set_ped_heading(me, a.angle, false));
                self.note(format!("SetPlayerFacingAngle {}", a.angle));
            }
            P::SetPlayerHealth(h) => {
                report(self, "set_ped_health", game.set_ped_health(me, h.health));
                self.note(format!("SetPlayerHealth {}", h.health));
            }
            P::SetPlayerArmour(a) => {
                report(self, "set_ped_armour", game.set_ped_armour(me, a.armour));
                self.note(format!("SetPlayerArmour {}", a.armour));
            }
            P::GivePlayerWeapon(w) => {
                report(
                    self,
                    "give_weapon",
                    game.give_weapon(me, WeaponType(w.weapon as u8), w.ammo),
                );
                self.note(format!("GivePlayerWeapon {} x{}", w.weapon, w.ammo));
            }
            P::ResetPlayerWeapons(_) => report(self, "clear_weapons", game.clear_weapons(me)),
            P::GivePlayerMoney(m) => {
                report(self, "add_score", game.add_score(m.amount));
                self.note(format!("GivePlayerMoney {}", m.amount));
            }
            P::ResetPlayerMoney(_) => {
                let m = game.player_money();
                if m != 0 {
                    report(self, "add_score", game.add_score(-m));
                }
            }
            P::SetPlayerTime(t) => {
                report(self, "set_clock", game.set_clock(t.hour, t.minute));
                self.note(format!("SetPlayerTime {:02}:{:02}", t.hour, t.minute));
            }
            P::SetPlayerWeather(w) => {
                report(self, "set_weather", game.set_weather(w.weather));
                self.note(format!("SetPlayerWeather {}", w.weather));
            }
            P::SetPlayerInterior(i) => {
                report(self, "set_area", game.set_area(AreaCode(i.interior)));
                report(self, "set_ped_area", game.set_ped_area(me, AreaCode(i.interior)));
                self.note(format!("SetPlayerInterior {}", i.interior));
            }
            P::TogglePlayerControllable(c) => {
                report(self, "set_player_control", game.set_player_control(c.enable != 0));
                self.note(format!("TogglePlayerControllable {}", c.enable));
            }
            P::SetPlayerCameraPosition(c) => {
                report(
                    self,
                    "set_fixed_position",
                    game.set_fixed_position(v3(c.pos), Vec3::default()),
                );
                self.note("SetPlayerCameraPosition".into());
            }
            P::SetPlayerCameraLookAt(c) => {
                let cut = if c.cut == 2 {
                    CameraCut::Cut
                } else {
                    CameraCut::Smooth
                };
                report(self, "point_at", game.point_at(v3(c.pos), cut));
                self.note("SetPlayerCameraLookAt".into());
            }
            P::SetPlayerCameraBehindPlayer(_) => report(self, "behind_player", game.behind_player()),
            other => self.note(format!("player rpc {} (not applied natively yet)", other.id())),
        }
    }

    /// The chat, dialog and game-text RPCs through the engine-drawn UI (`GameAdapter::ui`, docs/samp/overlay.md §5).
    fn on_ui_rpc(&mut self, rpc: ui::Incoming, game: &mut dyn GameAdapter) {
        use ui::Incoming as U;
        let note = match &rpc {
            U::SendClientMessage(m) => format!("chat: {}", text(&m.message)),
            U::PlayerChatMessage(m) => format!("chat {}: {}", m.player_id, text(&m.message)),
            U::ShowDialog(d) => format!("dialog {} \"{}\"", d.dialog_id, text(&d.title)),
            U::SendGameText(t) => format!("game text \"{}\"", text(&t.text)),
            other => format!("ui rpc {} (not shown natively yet)", other.id()),
        };
        self.note(note);
        let Some(ui) = game.ui() else { return };
        let msg = |kind, name: String, name_colour, text: String, colour| ChatMessage {
            kind,
            name,
            name_colour,
            text,
            colour,
            time: clock_now(),
        };
        match rpc {
            U::SendClientMessage(m) => ui.chat_add(&msg(
                ChatKind::Info,
                String::new(),
                Rgba::from_rgba_u32(m.colour),
                text(&m.message),
                Rgba::from_rgba_u32(m.colour),
            )),
            U::PlayerChatMessage(m) => {
                let name = self
                    .names
                    .get(&m.player_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Player{}", m.player_id));
                let white = Rgba::from_rgba_u32(0xFFFF_FFFF);
                ui.chat_add(&msg(ChatKind::Player, name, white, text(&m.message), white));
            }
            U::ShowDialog(d) => {
                if d.dialog_id < 0 {
                    ui.show_dialog(None);
                } else {
                    ui.show_dialog(Some(&DialogSpec {
                        id: d.dialog_id,
                        style: DialogStyle::from_byte(d.style),
                        title: text(&d.title),
                        body: text(&d.body),
                        button1: text(&d.button1),
                        button2: text(&d.button2),
                    }));
                }
            }
            U::SendGameText(t) => ui.game_text(&text(&t.text), t.time_ms, t.style as u8),
            _ => {}
        }
    }

    /// What the player did in the UI: chat lines and commands (RPC 101 / 50) and dialog answers (RPC 62).
    fn ui_events(&mut self, game: &mut dyn GameAdapter) {
        let Some(ui) = game.ui() else { return };
        for e in ui.poll_events() {
            match e {
                UiEvent::ChatSubmit(line) => {
                    let bytes = line
                        .chars()
                        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                        .collect::<Vec<_>>();
                    if line.starts_with('/') {
                        self.net.send_rpc(
                            &ui::PlayerRequestCommandMessage { command: bytes },
                            Reliability::Reliable,
                        );
                    } else {
                        self.net.send_rpc(
                            &ui::PlayerRequestChatMessage { message: bytes },
                            Reliability::Reliable,
                        );
                    }
                    self.note(format!("sent chat line \"{line}\""));
                }
                UiEvent::DialogResponse {
                    id,
                    button,
                    list_item,
                    text,
                } => {
                    let r = ui::OnPlayerDialogResponse {
                        dialog_id: id as u16,
                        response: button,
                        list_item,
                        text: text
                            .chars()
                            .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                            .collect(),
                    };
                    self.net.send_rpc(&r, Reliability::ReliableOrdered);
                    self.note(format!("dialog {id} answered with button {button}"));
                }
                _ => {}
            }
        }
    }

    fn on_vehicle_rpc(&mut self, rpc: vehicle::Incoming, game: &mut dyn GameAdapter) {
        use vehicle::Incoming as V;
        match rpc {
            V::StreamInVehicle(v) => {
                let model = ModelId(v.model as i32);
                report(self, "request_model", game.request_model(model));
                report(self, "load_all_now", game.load_all_now());
                let colour = |c: u8| if c == 0xFF { 0 } else { c };
                let spawn = VehicleSpawn {
                    model,
                    pos: Vec3::new(v.position.x, v.position.y, v.position.z),
                    heading_deg: v.angle,
                    colours: [colour(v.colour1), colour(v.colour2)],
                    paintjob: 3,
                    area: AreaCode(v.interior),
                };
                match game.create_vehicle(&spawn) {
                    Ok(r) => {
                        report(self, "set_vehicle_health", game.set_vehicle_health(r, v.health));
                        self.vehicles.insert(v.vehicle_id, r);
                        self.note(format!(
                            "vehicle {} (model {}) streamed in at ({:.1},{:.1},{:.1}) as {:#x}",
                            v.vehicle_id, v.model, v.position.x, v.position.y, v.position.z, r.0
                        ));
                    }
                    Err(e) => self.note(format!("vehicle {} (model {}): {e}", v.vehicle_id, v.model)),
                }
            }
            V::StreamOutVehicle(v) => {
                if let Some(r) = self.vehicles.remove(&v.vehicle_id) {
                    report(self, "destroy_vehicle", game.destroy_vehicle(r));
                    self.note(format!("vehicle {} streamed out", v.vehicle_id));
                }
            }
            V::PutPlayerInVehicle(p) => {
                let (Some(me), Some(&v)) = (game.local_player(), self.vehicles.get(&p.vehicle_id)) else {
                    self.note(format!("PutPlayerInVehicle {}: unknown vehicle", p.vehicle_id));
                    return;
                };
                let seat = (p.seat != 0).then(|| i32::from(p.seat) - 1);
                report(self, "warp_into_vehicle", game.warp_into_vehicle(me, v, seat));
                self.note(format!("PutPlayerInVehicle {} seat {}", p.vehicle_id, p.seat));
            }
            V::RemovePlayerFromVehicle(_) => {
                if let Some(me) = game.local_player()
                    && let Ok(s) = game.ped_snapshot(me)
                {
                    let p = s.matrix.pos;
                    report(
                        self,
                        "warp_out_of_vehicle",
                        game.warp_out_of_vehicle(me, Vec3::new(p.x + 2.0, p.y, p.z)),
                    );
                    self.note("RemovePlayerFromVehicle".into());
                }
            }
            other => self.note(format!("vehicle rpc {} (not applied natively yet)", other.id())),
        }
    }

    /// Driver sync (200) while the local player drives a vehicle the server streamed in.
    fn car_sync(
        &mut self,
        game: &mut dyn GameAdapter,
        v: VehicleRef,
        s: &sa_samp_api::peds::PedSnapshot,
        now: Instant,
    ) {
        let Some((&id, _)) = self.vehicles.iter().find(|(_, r)| **r == v) else {
            return;
        };
        let Ok(vs) = game.vehicle_snapshot(v) else { return };
        if vs.driver != game.local_player() {
            return;
        }
        let due = self
            .last_car_sync
            .is_none_or(|t| now.duration_since(t) >= Duration::from_millis(u64::from(self.on_foot_rate)));
        if !due {
            return;
        }
        self.last_car_sync = Some(now);
        let (pad, _) = game.pad();
        let a = |v: Vec3| [v.x, v.y, v.z];
        let m = vs.matrix;
        let sample = samp_client::vehicle::DriverSample {
            vehicle_id: id,
            left_right: pad.left_x,
            up_down: pad.left_y,
            keys: 0,
            additional_key: 0,
            rotation: Rotation {
                right: a(m.right),
                forward: a(m.forward),
                up: a(m.up),
            },
            position: a(m.pos),
            velocity: a(vs.move_speed),
            vehicle_health: vs.health,
            health: s.health,
            armour: s.armour,
            weapon: s.weapon.0,
            siren: vs.siren,
            landing_gear: vs.gear_down,
            trailer_id: 0,
            extra: 0,
        };
        let p = samp_client::vehicle::build_driver_sync(&sample, self.car_rotation);
        self.car_rotation = Some(p.rotation);
        self.net.send_sync(&p);
        self.syncs_sent += 1;
    }

    /// samp's local spawn routine, reduced to what the native adapter serves: control, HUD, the spawn
    /// point and heading, health, the spawn weapons, the camera, then RPC 52 PlayerSpawn.
    fn spawn_local(&mut self, game: &mut dyn GameAdapter) {
        let (Some(s), Some(me)) = (self.spawn, game.local_player()) else {
            self.note("spawn: no spawn info or no local player".into());
            return;
        };
        report(self, "restore", game.restore());
        report(self, "behind_player", game.behind_player());
        report(self, "set_hud_visible", game.set_hud_visible(true));
        report(self, "set_player_control", game.set_player_control(true));
        report(
            self,
            "request_collision",
            game.request_collision(Vec3::new(s.pos.x, s.pos.y, s.pos.z)),
        );
        report(
            self,
            "set_restart_point",
            game.set_restart_point(Vec3::new(s.pos.x, s.pos.y, s.pos.z), s.rotation),
        );
        report(self, "clear_weapons", game.clear_weapons(me));
        for (w, a) in s.weapons.iter().zip(s.ammo) {
            if *w > 0 && *w < 47 {
                report(self, "give_weapon", game.give_weapon(me, WeaponType(*w as u8), a));
            }
        }
        report(self, "set_ped_health", game.set_ped_health(me, 100.0));
        report(
            self,
            "set_ped_position",
            game.set_ped_position(me, Vec3::new(s.pos.x, s.pos.y, s.pos.z + 0.5)),
        );
        report(
            self,
            "set_ped_heading",
            game.set_ped_heading(me, s.rotation, false),
        );
        if s.skin != 0 && game.model_loaded(ModelId(s.skin as i32)) {
            report(
                self,
                "set_ped_model",
                game.set_ped_model(me, ModelId(s.skin as i32)),
            );
        }
        self.spawned = true;
        self.sender.reset();
        self.net
            .send_rpc(&player::PlayerSpawn, Reliability::ReliableOrdered);
        self.note(format!(
            "spawned at ({:.2},{:.2},{:.2}) heading {} skin {} (skin change natively: later)",
            s.pos.x, s.pos.y, s.pos.z, s.rotation, s.skin
        ));
    }

    /// On-foot sync (207) when due, stats sync (205) every second.
    fn sync(&mut self, game: &mut dyn GameAdapter, now: Instant) {
        let Some(me) = game.local_player() else { return };
        let Ok(s) = game.ped_snapshot(me) else { return };
        if let Some(v) = s.vehicle {
            self.car_sync(game, v, &s, now);
            return;
        }
        let (pad, _) = game.pad();
        let pad = footsync::PadState {
            left_stick_x: pad.left_x,
            left_stick_y: pad.left_y,
            right_stick_x: pad.right_x,
            right_stick_y: pad.right_y,
            left_shoulder1: pad.shoulders[0],
            left_shoulder2: pad.shoulders[1],
            right_shoulder1: pad.shoulders[2],
            right_shoulder2: pad.shoulders[3],
            dpad_up: pad.dpad[0],
            dpad_down: pad.dpad[1],
            dpad_left: pad.dpad[2],
            dpad_right: pad.dpad[3],
            start: pad.start_select[0],
            select: pad.start_select[1],
            square: pad.face[0],
            triangle: pad.face[1],
            cross: pad.face[2],
            circle: pad.face[3],
            shock_left: pad.shock[0],
            shock_right: pad.shock[1],
            chat_indicated: pad.chat_indicated,
            ped_walk: pad.ped_walk,
            vehicle_mouse_look: 0,
            radio_track_skip: 0,
        };
        let m = s.matrix;
        let a = |v: Vec3| [v.x, v.y, v.z];
        let sample = LocalSample {
            pad,
            key_ctx: KeyContext {
                in_vehicle: false,
                can_fire: true,
                jetpack: false,
            },
            position: a(m.pos),
            rotation: Rotation {
                right: a(m.right),
                forward: a(m.forward),
                up: a(m.up),
            },
            health: s.health,
            armour: s.armour,
            weapon: s.weapon.0,
            special_action: 0,
            move_speed: a(s.move_speed),
            animation: 0,
            surf_offset: [0.0; 3],
            surf_id: 0xFFFF,
        };
        let keys = footsync::key_word(&sample.pad, sample.key_ctx);
        let interval = footsync::on_foot_interval(self.on_foot_rate, 0);
        if self.sender.due(
            now,
            interval,
            keys,
            sample.pad.left_stick_x,
            sample.pad.left_stick_y,
        ) {
            let f = footsync::build_foot_sync(&sample, self.sender.last_rotation());
            if let Some(f) = self.sender.offer(now, f) {
                self.net.send_sync(&f);
                self.syncs_sent += 1;
            }
        }
        if self
            .last_stats
            .is_none_or(|t| now.duration_since(t) > footsync::STATS_INTERVAL)
        {
            self.last_stats = Some(now);
            self.net.send_sync(&StatsSync {
                money: game.player_money(),
                drunk_level: 0,
            });
        }
    }
}

/// SA:MP text (code page 1252 bytes) as a `String`, byte for byte (Latin-1 view, the UI's font covers it).
fn text(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

/// The local wall-clock time (UTC hour, minute, second) for chat timestamps.
fn clock_now() -> [u8; 3] {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    [((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8]
}

fn other_id(i: &Incoming) -> String {
    match i {
        Incoming::Remote(r) => format!("{} (remote)", r.id()),
        Incoming::Session(r) => format!("{} (session)", r.id()),
        Incoming::Ui(r) => format!("{} (ui)", r.id()),
        Incoming::Vehicle(r) => format!("{} (vehicle)", r.id()),
        Incoming::World(r) => format!("{} (world)", r.id()),
        Incoming::Player(r) => format!("{} (player)", r.id()),
        Incoming::InitGame(_) => "139 (InitGame)".into(),
        Incoming::ConnectionRejected { reason } => format!("130 (rejected {reason})"),
    }
}

/// Logs an adapter call that failed (an unported path), once per call site and error.
fn report(h: &NetHooks, what: &str, r: sa_samp_api::GameResult<()>) {
    if let Err(e) = r {
        h.note(format!("adapter {what}: {e}"));
    }
}

impl GameHooks for NetHooks {
    fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {
        if phase == FramePhase::BeforeGameProcess {
            self.tick(game);
        }
    }

    fn shutdown(&mut self, _game: &mut dyn GameAdapter) {
        self.net.disconnect(Instant::now());
        self.note("disconnected".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_valid_serial_seed_only() {
        let s = "0123456789ABCDEF0123456789abcdef01234567";
        assert_eq!(parse_seed(&format!("x=1\nserial_seed={s}\n")).as_deref(), Some(s));
        assert_eq!(parse_seed("serial_seed=123"), None);
        assert_eq!(parse_seed(""), None);
    }
}
