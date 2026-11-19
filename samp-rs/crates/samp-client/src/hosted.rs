//! samp-rs's client on a game that serves `sa_samp_api::GameAdapter` (the game's mod API, through the SA:MP mod):
//! samp's netgame on [`NetClient`] and the sync builders of `footsync`, driving the local player through the
//! adapter. It joins, runs class selection, spawns, applies the player RPCs a gamemode uses on its own player
//! (position, facing, health, armour, weapons, money, clock, weather, interior, control, camera, skin, animations),
//! streams the server's vehicles and the other players in and out, and sends on-foot, driver and stats sync.
//!
//! The server's pickups, actors, checkpoints, gang zones and map icons are `serverworld`'s records, as on the DLL
//! client. The connection lifecycle is samp's ([`NetGame`]): a server restart (RPC 40) and a lost connection reset the
//! world the server made and print samp's lines, a lost connection reconnects with samp's timing, and the next InitGame
//! starts class selection again.
//!
//! The UI is samp-rs's own ([`UiHost`]: `UiCore` fed with the RPCs, drawn through the adapter's UI backend).
//! Everything runs on the game thread inside the frame callback; the host gives the session its transport and the
//! clock (`samp_proto::time`).

use std::collections::HashMap;
use std::time::Duration;

use samp_proto::time::Instant;

use crate::classsel::{self, ClassSelection};
use crate::footsync::{self, FootSender, KeyContext, LocalSample, Rotation};
use crate::hostedobjects::{HostedObjects, PlayerPeds};
use crate::lifecycle::{self, NetGame};
use crate::netclient::{Event, NetClient};
use crate::serverworld::ServerWorld;
use crate::ui::host::UiHost;
use crate::ui::render::TagDraw;
use samp_proto::raknet::Reliability;
use samp_proto::rpcs::{Incoming, player, session, world};
use samp_proto::sync::player::StatsSync;
use samp_ui::classpanel::ClassButton;
use samp_ui::text::NoExpansion;

use sa_samp_api::camera::CameraCut;
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::input::{EventResult, WindowEvent};
use sa_samp_api::peds::{AnimRequest, BoneId, PedTask};
use sa_samp_api::world::LineFilter;
use sa_samp_api::{AreaCode, GameAdapter, ModelId, PedRef, Vec3, VehicleRef, WeaponType};

mod death;
mod trains;
mod vehicles;

/// Where a hosted client's lines go (the host's log).
pub type Log = Box<dyn Fn(&str)>;

/// What the host tells the client at start.
pub struct HostedConfig {
    /// The server as the user named it (for samp's `Connecting to` line).
    pub host: String,
    /// The server's port.
    pub port: u16,
    /// Our player name.
    pub name: String,
    /// Press Shift in class selection by itself (headless runs have no keyboard; samp's 2 s guard still applies).
    pub auto_spawn: bool,
}

/// The hosted client's state.
pub struct HostedClient {
    net: NetClient,
    start: Instant,
    log: Log,
    connected: bool,
    on_foot_rate: u32,
    spawn: Option<player::SpawnInfo>,
    spawned: bool,
    sender: FootSender,
    last_stats: Option<Instant>,
    syncs_sent: u32,
    last_report: Option<Instant>,
    world_set_up: bool,
    /// Server vehicle id → the game's vehicle (streamed in by RPC 164).
    vehicles: HashMap<u16, VehicleRef>,
    /// What the client keeps of each of those vehicles.
    vehicle_state: HashMap<u16, vehicles::HostedVehicle>,
    /// InitGame's manual engine and lights mode.
    manual_engine: bool,
    /// The local player has spawned once (SCM events apply from then on).
    spawned_once: bool,
    /// Server player id → the ped standing in for that player (streamed in by RPC 32).
    players: HashMap<u16, PedRef>,
    /// What the nametags show of the other players: NPC (RPC 137), health, armour and the time of the last
    /// on-foot sync.
    remotes: HashMap<u16, RemoteTag>,
    last_car_sync: Option<Instant>,
    car_rotation: Option<samp_proto::types::Quat>,
    /// samp's netgame state and connect timer (`Connecting to` 3 s after `Started`).
    netgame: NetGame,
    /// samp's clock and weather settings: the time the server holds is applied every frame (RPC 30
    /// TogglePlayerClock lets the game's clock run instead; samp starts with it off).
    clock: crate::world::World,
    /// The local player uses the mobile phone (special action 11 until 0 / 13).
    phone: bool,
    /// The local player wears the jetpack (special action 2 until 0).
    jetpack: bool,
    /// samp's class selection (arrows, Shift; RPC 128 / 129).
    class_sel: ClassSelection,
    /// `HOST:PORT` for the `Connecting to` line.
    server_text: (String, u16),
    auto_spawn: bool,
    netgame_created: bool,
    /// Our player name (the netgame's).
    name: String,
    /// The server's pickups, actors, checkpoints, gang zones and map icons.
    world: ServerWorld,
    /// samp-rs's UI: chat, dialogs, textdraws, game text, scoreboard, class-selection panel.
    pub ui: UiHost,
    /// The server's objects and the buildings it removed.
    objects: HostedObjects,
    /// The local player's death and respawn.
    life: death::LocalLife,
    /// The server's trains.
    trains: trains::Trains,
}

/// The keys class selection polls (Windows virtual-key codes): left, right, and either Shift key (samp asks for
/// VK_SHIFT).
const KEY_LEFT: u16 = 0x25;
const KEY_RIGHT: u16 = 0x27;
const KEY_SHIFTS: [u16; 3] = [0x10, 0xA0, 0xA1];
/// samp's not-connected scene (samp-rs `connecting_tick`, samp.dll 0x100089B0): the ped parked here, the
/// camera fixed at `CONNECT_CAMERA` looking at `CONNECT_LOOK_AT` with a jump cut, weather 1, HUD off.
const CONNECT_PED_POS: Vec3 = Vec3::new(1133.05, -2038.40, 69.1);
const CONNECT_CAMERA: Vec3 = Vec3::new(1093.0, -2036.0, 90.0);
const CONNECT_LOOK_AT: Vec3 = Vec3::new(384.0, -1557.0, 20.0);
const CONNECT_WEATHER: u8 = 1;

impl HostedClient {
    /// The client on `net` (the session the host opened, its serial seed set); the first request goes out with the
    /// first frame.
    pub fn new(net: NetClient, config: HostedConfig, log: Log) -> Self {
        let now = Instant::now();
        Self {
            net,
            start: now,
            log,
            connected: false,
            on_foot_rate: 30,
            spawn: None,
            spawned: false,
            sender: FootSender::default(),
            last_stats: None,
            syncs_sent: 0,
            last_report: None,
            world_set_up: false,
            vehicles: HashMap::new(),
            vehicle_state: HashMap::new(),
            manual_engine: false,
            spawned_once: false,
            players: HashMap::new(),
            remotes: HashMap::new(),
            last_car_sync: None,
            car_rotation: None,
            netgame: NetGame::new(0),
            clock: crate::world::World::default(),
            phone: false,
            jetpack: false,
            class_sel: ClassSelection::new(now),
            server_text: (config.host, config.port),
            auto_spawn: config.auto_spawn,
            netgame_created: false,
            name: config.name,
            world: ServerWorld::default(),
            ui: UiHost::new(None),
            objects: HostedObjects::default(),
            life: death::LocalLife::default(),
            trains: trains::Trains::default(),
        }
    }

    /// The session.
    pub fn net(&self) -> &NetClient {
        &self.net
    }

    /// Whether the server accepted the join (InitGame arrived): class selection or the game from here on.
    pub fn connected(&self) -> bool {
        self.connected
    }

    fn note(&self, line: String) {
        (self.log)(&format!("{:>6} ms  {line}", self.start.elapsed().as_millis()));
    }

    fn tick(&mut self, game: &mut dyn GameAdapter) {
        let now = Instant::now();
        if !self.world_set_up {
            self.world_set_up = true;
            self.world_setup(game);
        }
        let now_ms = now.duration_since(self.start).as_millis() as u64;
        if !self.netgame_created {
            // samp creates its netgame (and prints `Started`) once the game runs; the connect tick counts from here.
            self.netgame_created = true;
            self.netgame = NetGame::new(now_ms);
            let (host, port) = self.server_text.clone();
            self.ui.core.on_netgame_start(&host, port, self.name.as_bytes());
            self.note(format!("chat (debug): {}", crate::ui::core::msg::STARTED));
        }
        if self.netgame.connect_due(now_ms) {
            let line = lifecycle::msg::connecting(&self.server_text.0, self.server_text.1);
            self.debug_line(&line);
            match self.net.connect() {
                Ok(()) => self.note(format!("{line} (connect tick)")),
                Err(e) => self.note(format!("connect: {e}")),
            }
            self.netgame
                .attempt_started(Instant::now().duration_since(self.start).as_millis() as u64);
        }
        for e in self.net.tick(now) {
            self.on_event(e, game);
        }
        // samp's netgame tick (samp-rs `world_frame`): the server's time of day while the server holds the time
        if let Some((h, m)) = self.clock.time_of_day() {
            report(self, "set_clock", game.set_clock(h, m));
        }
        if self.netgame.connected() {
            self.world_tick(game);
            self.vehicle_params_tick(game);
        }
        if !self.netgame.connected() {
            self.connecting_scene(game);
        } else if self.class_sel.is_active() {
            self.class_selection_tick(game, now);
        } else if self.alive_or_dead() {
            self.death_tick(game, now);
        }
        self.ui.core.class_selection = self.class_sel.is_active();
        self.ui_events();
        let peds = PlayerPeds { local_id: self.net.player_id(), remote: &self.players };
        self.objects.process(game, &self.vehicles, &peds);
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
            if self.objects.count() > 0 {
                self.note(self.objects.status(game));
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
                self.manual_engine = g.manual_engine_and_lights;
                self.note(format!(
                    "InitGame: server {:?}, our id {:?}, hour {}, weather {}, gravity {}, entry-exits disabled {}",
                    g.server_name,
                    self.net.player_id(),
                    g.world_time,
                    g.weather,
                    g.gravity,
                    g.disable_enter_exits
                ));
                self.clock.hour = g.world_time;
                self.clock.weather = g.weather;
                report(self, "set_clock", game.set_clock(g.world_time, 0));
                report(self, "set_weather", game.set_weather(g.weather));
                report(self, "set_gravity", game.set_gravity(g.gravity));
                report(self, "set_stunt_bonus", game.set_stunt_bonus(g.stunt_bonus));
                report(self, "set_zone_names", game.set_zone_names(false));
                report(self, "disable_entry_exits", crate::world::apply_enter_exits(game, &g));
                self.netgame.on_init_game();
                self.note(format!("chat (debug): Connected to {{B9C9BF}}{}", g.server_name));
                self.ui.core.on_init_game(&g);
                // samp enters class selection on InitGame; its first tick sends RPC 128 for class 0.
                self.enter_class_selection(game, g.spawns_available);
            }
            Event::Packet(p) => {
                self.note(format!("packet {p:?} in netgame state {}", self.netgame.state));
                let actions = self.netgame.on_packet(p);
                self.apply_actions(actions, game);
            }
            Event::Rejected(r) => {
                self.note(format!("connection rejected: {r}"));
                let actions = self.netgame.on_rejected(r);
                self.apply_actions(actions, game);
            }
            Event::Rpc(rpc) => {
                // The UI takes what is its (chat, dialogs, game text, textdraws, names, scores, bubbles, ...).
                self.ui.core.on_rpc(&rpc, &NoExpansion);
                match rpc {
                    Incoming::Player(p) => self.on_player_rpc(p, game),
                    Incoming::Vehicle(v) => self.on_vehicle_rpc(v, game),
                    Incoming::Ui(u) => self.note(format!("ui rpc {}", u.id())),
                    Incoming::Remote(r) => self.on_remote_rpc(r, game),
                    Incoming::World(w) if is_object_rpc(&w) => self.on_object_rpc(w, game),
                    Incoming::World(w) => self.on_world_rpc(w, game),
                    Incoming::Session(s) => self.on_session_rpc(s, game),
                    other => self.note(format!("rpc {} (not applied natively yet)", other_id(&other))),
                }
            }
            Event::Sync(s) => self.on_sync(s, game),
        }
    }

    /// Lifecycle actions (lifecycle.rs), in order.
    fn apply_actions(&mut self, actions: Vec<lifecycle::Action>, game: &mut dyn GameAdapter) {
        use lifecycle::Action as A;
        for a in actions {
            match a {
                A::Debug(t) => self.debug_line(t),
                A::Info(t) => self.info_line(t),
                A::Disconnect(_) => self.net.disconnect_silent(),
                A::Reset => self.reset_routine(game),
                A::DeletePlayers => {
                    let ids: Vec<u16> = self.players.keys().copied().collect();
                    for id in ids {
                        self.remove_player(id, game);
                    }
                    self.remotes.clear();
                    self.ui.core.forget_players();
                }
                // samp-rs has no audio stream on this host
                A::StopAudio { .. } => {}
            }
        }
    }

    /// RPC 40 reset (docs/re/subsystems/session-misc.md, "40 PlayerClose"), on the adapter: the remote players'
    /// peds and what the server made removed, the local player back to not spawned, the UI's pools and the server
    /// settings back to their defaults, interior 0, money 0, the local ped killed, then the not-connected scene every
    /// frame until InitGame. Mirrors samp.dll B 0x1000A530 (the DLL's `client/session.rs` on the original
    /// executable).
    fn reset_routine(&mut self, game: &mut dyn GameAdapter) {
        self.note("reset routine (server restart / lost connection)".into());
        let ids: Vec<u16> = self.players.keys().copied().collect();
        for id in ids {
            self.remove_player(id, game);
        }
        self.class_sel.leave();
        self.spawned = false;
        self.phone = false;
        self.jetpack = false;
        self.clear_server_entities(game);
        self.ui.core.on_reset();
        self.ui.core.colours = crate::ui::colours::PlayerColours::new();
        // the server settings' defaults: 12:00, weather 1, the clock stopped
        self.clock.hour = 12;
        self.clock.minute = 0;
        self.clock.weather = crate::world::CONNECTING_WEATHER;
        self.clock.toggle_clock(false);
        if let Some(me) = game.local_player() {
            report(self, "set_area", game.set_area(AreaCode(0)));
            report(self, "set_ped_area", game.set_ped_area(me, AreaCode(0)));
            let m = game.player_money();
            if m != 0 {
                report(self, "add_score", game.add_score(-m));
            }
            report(self, "set_ped_health", game.set_ped_health(me, 0.0));
            report(self, "explode_head", game.explode_head(me));
            report(self, "set_ped_armour", game.set_ped_armour(me, 0.0));
        }
        report(self, "set_zone_names", game.set_zone_names(false));
        report(self, "set_gravity", game.set_gravity(lifecycle::DEFAULT_GRAVITY));
        {
            use sa_samp_api::config::RuntimeToggle;
            report(self, "set_toggle", game.set_toggle(RuntimeToggle::GameClock(false)));
        }
        self.info_line(lifecycle::msg::RESTARTING);
    }

    /// The pools samp deletes and recreates on a reset: vehicles, the server's objects, then the world's records.
    fn clear_server_entities(&mut self, game: &mut dyn GameAdapter) {
        self.clear_trains(game);
        for (_, v) in std::mem::take(&mut self.vehicles) {
            report(self, "destroy_vehicle", game.destroy_vehicle(v));
        }
        self.vehicle_state.clear();
        self.objects.clear(game);
        self.world.clear(game);
    }

    /// The world RPCs ([`ServerWorld`]).
    fn on_world_rpc(&mut self, rpc: world::Incoming, game: &mut dyn GameAdapter) {
        let id = rpc.id();
        let colours = &self.ui.core.colours;
        let colour = |c: u32| crate::worldents::radar_trace_colour(c, |id| colours.rgba(id));
        if let Some(line) = self.world.on_rpc(game, &rpc, &colour) {
            self.debug_line(&line);
        }
        self.note(format!("world rpc {id}"));
    }

    /// Per frame while connected: the pickups the local player touches (samp's collision path), then the
    /// checkpoints and the pickup poll.
    fn world_tick(&mut self, game: &mut dyn GameAdapter) {
        for id in self.world.touched_pickups(game) {
            let before = self.world.sent_131;
            self.world.report_touch(&mut self.net, id);
            if self.world.sent_131 != before {
                self.note(format!("pickup {id} picked up"));
            }
        }
        let colours = &self.ui.core.colours;
        let colour = |c: u32| crate::worldents::radar_trace_colour(c, |id| colours.rgba(id));
        self.world.tick(game, &mut self.net, &colour);
    }

    /// The session RPCs: RPC 40 (gmx) resets and waits for InitGame; the drunk visuals (RPC 92).
    fn on_session_rpc(&mut self, rpc: session::Incoming, game: &mut dyn GameAdapter) {
        use session::Incoming as S;
        match rpc {
            S::PlayerClose(_) => {
                self.note("server restart (RPC 40)".into());
                let actions = self.netgame.on_server_restart();
                self.apply_actions(actions, game);
            }
            S::SetPlayerDrunkVisuals(v) => {
                if game.local_player().is_some() {
                    report(self, "set_drunkenness", game.set_drunkenness(v.value as i32));
                }
            }
            S::SetVehicleTyreStatus(t) => self.set_vehicle_tyres(t.vehicle_id, t.tyres, game),
            S::SetVehicleParamsForPlayerOld(o) => self.set_vehicle_doors_old(o.vehicle_id, o.doors_locked, game),
            other => self.note(format!("session rpc {} (not applied natively yet)", other.id())),
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
                if r.selectable != 0 {
                    // samp: the spawn info is stored, the ped loses its weapons and shows the class skin.
                    self.store_spawn_info(r.spawn);
                    report(self, "clear_weapons", game.clear_weapons(me));
                    self.set_skin(game, r.spawn.skin as i32);
                }
                self.class_sel.on_class_response(r.selectable != 0);
            }
            P::SetSpawnInfo(s) => self.store_spawn_info(s.spawn),
            P::ForcePlayerClassSelection(_) => self.force_class_selection(),
            P::PlayerRequestSpawnResponse(r) => {
                self.note(format!("spawn response {}", r.response));
                if self.class_sel.on_spawn_response(r.response) {
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
                // samp arms the given weapon (not forced).
                report(
                    self,
                    "set_current_weapon",
                    game.set_current_weapon(me, WeaponType(w.weapon as u8)),
                );
                self.note(format!("GivePlayerWeapon {} x{}", w.weapon, w.ammo));
            }
            P::SetPlayerArmedWeapon(w) => {
                report(
                    self,
                    "set_current_weapon",
                    game.set_current_weapon(me, WeaponType(w.weapon as u8)),
                );
                self.note(format!("SetPlayerArmedWeapon {}", w.weapon));
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
                (self.clock.hour, self.clock.minute) = (t.hour, t.minute);
                self.note(format!("SetPlayerTime {:02}:{:02}", t.hour, t.minute));
            }
            P::SetPlayerWorldTime(t) => self.clock.hour = t.hour,
            P::SetPlayerWeather(w) => {
                report(self, "set_weather", game.set_weather(w.weather));
                self.note(format!("SetPlayerWeather {}", w.weather));
            }
            P::SetPlayerInterior(i) => {
                report(self, "set_area", game.set_area(AreaCode(i.interior)));
                report(self, "set_ped_area", game.set_ped_area(me, AreaCode(i.interior)));
                if let Ok(s) = game.ped_snapshot(me) {
                    report(self, "request_collision", game.request_collision(s.matrix.pos));
                }
                self.note(format!("SetPlayerInterior {}", i.interior));
            }
            P::SetPlayerPositionFindZ(p) => {
                let z = game.ground_z(v3(p.pos)) + crate::behaviour::FIND_Z_ABOVE;
                report(self, "set_ped_position", game.set_ped_position(me, Vec3::new(p.pos.x, p.pos.y, z)));
                self.note(format!("SetPlayerPositionFindZ ({:.2},{:.2},{z:.2})", p.pos.x, p.pos.y));
            }
            P::CreateExplosion(e) => {
                let shake = f32::from_bits(crate::combat::explosion_shake_bits(e.radius));
                report(self, "add_explosion", game.add_explosion(v3(e.pos), e.kind as i32, e.radius, shake));
                self.note(format!("CreateExplosion type {} radius {}", e.kind, e.radius));
            }
            P::InterpolateCamera(c) => {
                if let Some(cut) = crate::behaviour::interpolation_cut(c.time, c.cut) {
                    let cut = if cut == 2 { CameraCut::Cut } else { CameraCut::Smooth };
                    let r = game.interpolate(!c.is_position, v3(c.from), v3(c.to), c.time, cut);
                    report(self, "interpolate", r);
                }
                self.note(format!("InterpolateCamera position={} {} ms", c.is_position, c.time));
            }
            P::ToggleWidescreen(w) => {
                self.debug_line(&crate::behaviour::widescreen_line(w.enable));
                report(self, "set_widescreen", game.set_widescreen(w.enable != 0));
            }
            P::SetPlayerGravity(g) => {
                report(self, "set_gravity", game.set_gravity(g.gravity));
                self.note(format!("SetPlayerGravity {}", g.gravity));
            }
            P::SetPlayerVelocity(v) => report(self, "set_ped_move_speed", game.set_ped_move_speed(me, v3(v.velocity))),
            P::SetPlayerSkillLevel(k) if self.is_me(u32::from(k.player_id)) => {
                if let Some(stat) = crate::combat::weapon_skill_stat(k.skill_type) {
                    report(self, "set_float_stat", game.set_float_stat(stat, f32::from(k.level)));
                }
            }
            P::SetPlayerFightingStyle(f) => {
                let ped = if self.is_me(u32::from(f.player_id)) { Some(me) } else { self.players.get(&f.player_id).copied() };
                if let Some(ped) = ped {
                    let r = game.set_fighting_style(ped, f.style, crate::behaviour::FIGHTING_STYLE_MOVES);
                    report(self, "set_fighting_style", r);
                }
            }
            P::SetPlayerShopName(n) => {
                let (name, loads) = crate::behaviour::shop_name(&n.name);
                if loads {
                    report(self, "load_shop", game.load_shop(&text(name)));
                }
                self.note(format!("SetPlayerShopName {:?}", text(name)));
            }
            // RPC 48: stored in the custom-model manager, which R4 never creates; nothing happens
            P::SetPlayerVirtualWorld(w) => self.note(format!("SetPlayerVirtualWorld {} (no effect in R4)", w.world_id)),
            P::TogglePlayerControllable(c) => {
                report(self, "set_player_control", game.set_player_control(c.enable != 0));
                self.note(format!("TogglePlayerControllable {}", c.enable));
            }
            P::SetPlayerCameraPosition(c) => {
                report(self, "camera position", crate::camops::fixed_position(game, v3(c.pos)));
                self.note("SetPlayerCameraPosition".into());
            }
            P::SetPlayerCameraLookAt(c) => {
                let cut = if crate::behaviour::camera_cut(c.cut) == 2 { CameraCut::Cut } else { CameraCut::Smooth };
                report(self, "camera look-at", crate::camops::look_at(game, v3(c.pos), cut));
                self.note("SetPlayerCameraLookAt".into());
            }
            P::SetPlayerCameraBehindPlayer(_) => report(self, "camera behind", crate::camops::behind_player(game)),
            P::SetPlayerSpecialAction(a) => {
                // samp.dll 0x100030F0 (samp-rs `on_special_action`) for the phone and hands up: 0 / 13 stop the
                // phone, 11 starts it and 10 raises the hands (TASK_HANDS_UP for ever), both on foot only.
                self.note(format!("SetPlayerSpecialAction {}", a.action));
                let in_vehicle = game.ped_snapshot(me).is_ok_and(|s| s.vehicle.is_some());
                if matches!(a.action, 0 | 13) && self.phone {
                    self.phone = false;
                    report(self, "start_task", game.start_task(me, PedTask::UseMobilePhone(false)));
                }
                match a.action {
                    11 if !in_vehicle => {
                        let r = game.start_task(me, PedTask::UseMobilePhone(true));
                        self.phone = r.is_ok();
                        report(self, "start_task", r);
                    }
                    10 if !in_vehicle && game.ped_in_world(me) => {
                        report(self, "start_task", game.start_task(me, PedTask::HandsUp { time_ms: -2 }));
                    }
                    2 if !in_vehicle => {
                        let r = game.set_jetpack(me, true);
                        self.jetpack = r.is_ok();
                        report(self, "set_jetpack", r);
                    }
                    0 if self.jetpack => {
                        self.jetpack = false;
                        report(self, "set_jetpack", game.set_jetpack(me, false));
                    }
                    _ => {}
                }
            }
            P::EnableStuntBonusForPlayer(e) => {
                report(self, "set_stunt_bonus", game.set_stunt_bonus(e.enable));
                self.note(format!("EnableStuntBonusForPlayer {}", e.enable));
            }
            P::TogglePlayerClock(c) => {
                use sa_samp_api::config::RuntimeToggle;
                self.clock.on_toggle_clock_rpc(c.enable != 0, game.clock());
                report(self, "set_toggle", game.set_toggle(RuntimeToggle::GameClock(c.enable != 0)));
                self.note(format!("TogglePlayerClock {}", c.enable));
            }
            P::SetPlayerSkin(s) => {
                self.note(format!("SetPlayerSkin {} {}", s.player_id, s.skin));
                if self.is_me(s.player_id) {
                    self.set_skin(game, s.skin as i32);
                }
            }
            P::PlayerPlaySound(s) => {
                self.note(format!("PlayerPlaySound {}", s.sound_id));
                report(self, "one_off_sound", game.one_off_sound(v3(s.pos), s.sound_id as i32));
            }
            P::ApplyPlayerAnimation(a) => {
                let lib = text(&a.lib);
                let name = text(&a.name);
                self.note(format!("ApplyPlayerAnimation {} {lib}/{name}", a.player_id));
                // samp ignores the "SEX" library.
                if self.is_me(u32::from(a.player_id)) && !lib.eq_ignore_ascii_case("SEX") {
                    let anim = AnimRequest {
                        name: &name,
                        block: &lib,
                        delta: a.delta,
                        looped: a.looping,
                        lock_x: a.lock_x,
                        lock_y: a.lock_y,
                        freeze: a.freeze,
                        time_ms: a.time as i32,
                    };
                    report(self, "play_anim", game.play_anim(me, &anim));
                }
            }
            P::ClearPlayerTasks(c) => {
                self.note(format!("ClearPlayerTasks {}", c.player_id));
                if self.is_me(u32::from(c.player_id)) {
                    report(self, "stop_anims", game.stop_anims(me));
                }
            }
            P::RemoveBuildingForPlayer(b) => {
                let centre = Vec3::new(b.pos.x, b.pos.y, b.pos.z);
                let line = self.objects.remove_building(b.model_id as i32, centre, b.radius, game);
                self.note(line);
            }
            // RPC 60: the game time follows the server's while the clock is off
            P::SendGameTimeUpdate(t) if !self.clock.clock_enabled => {
                report(self, "set_game_time_ms", game.set_game_time_ms((t.time as u32) & 0x3FFF_FFFF));
            }
            P::SendGameTimeUpdate(_) => {}
            // teams only cancel same-team damage, which this client does not simulate yet
            P::SetPlayerTeam(t) => self.note(format!("SetPlayerTeam {} {}", t.player_id, t.team)),
            // the UI's player colours take RPC 72 (chat, nametags, scoreboard)
            P::SetPlayerColor(_) => {}
            other => self.note(format!("player rpc {} (not applied natively yet)", other.id())),
        }
    }

    /// The server's objects (`hostedobjects`).
    fn on_object_rpc(&mut self, rpc: samp_proto::rpcs::world::Incoming, game: &mut dyn GameAdapter) {
        let id = rpc.id();
        let peds = PlayerPeds { local_id: self.net.player_id(), remote: &self.players };
        let line = self.objects.on_rpc(rpc, game, &peds);
        self.note(line.unwrap_or_else(|| format!("object rpc {id}: no such object or player")));
    }

    /// The other players: a ped stands in for each player the server streams in, placed by its on-foot sync.
    fn on_remote_rpc(&mut self, rpc: samp_proto::rpcs::remote::Incoming, game: &mut dyn GameAdapter) {
        use samp_proto::rpcs::remote::Incoming as R;
        match rpc {
            R::PlayerStreamIn(p) => {
                self.remove_player(p.player_id, game);
                let model = if game.is_ped_model(ModelId(p.skin as i32)) { p.skin as i32 } else { 0 };
                let pos = Vec3::new(p.position.x, p.position.y, p.position.z);
                match game.create_actor(ModelId(model), pos, p.facing_angle) {
                    Ok(ped) => {
                        self.players.insert(p.player_id, ped);
                        self.note(format!(
                            "player {} (skin {}) streamed in at ({:.1},{:.1},{:.1}) as {:#x}",
                            p.player_id, p.skin, pos.x, pos.y, pos.z, ped.0
                        ));
                    }
                    Err(e) => self.note(format!("player {} (skin {}): {e}", p.player_id, p.skin)),
                }
            }
            R::PlayerStreamOut(p) => {
                self.remove_player(p.player_id, game);
                self.note(format!("player {} streamed out", p.player_id));
            }
            R::PlayerQuit(p) => {
                self.remove_player(p.player_id, game);
                self.remotes.remove(&p.player_id);
                self.note(format!("player {} left", p.player_id));
            }
            R::PlayerJoin(j) => {
                self.remotes.insert(j.player_id, RemoteTag { npc: j.is_npc != 0, ..RemoteTag::default() });
            }
            _ => {}
        }
    }

    fn remove_player(&mut self, id: u16, game: &mut dyn GameAdapter) {
        if let Some(ped) = self.players.remove(&id) {
            report(self, "destroy_ped", game.destroy_ped(ped));
        }
    }

    /// The other players' on-foot sync: their stand-in peds take the position and the heading.
    fn on_sync(&mut self, sync: samp_proto::sync::IncomingSync, game: &mut dyn GameAdapter) {
        let samp_proto::sync::IncomingSync::Foot { sync, .. } = sync else { return };
        let Some(&ped) = self.players.get(&sync.player_id) else { return };
        let f = crate::remote::FootData::from_sync(&sync);
        let tag = self.remotes.entry(sync.player_id).or_default();
        (tag.health, tag.armour, tag.synced) = (f.health, f.armour, Some(Instant::now()));
        let forward = crate::remote::rows_from_quat(f.rotation).forward;
        let heading = crate::remote::heading_from_forward(forward).to_degrees();
        let [x, y, z] = f.position;
        report(self, "set_ped_position", game.set_ped_position(ped, Vec3::new(x, y, z)));
        report(self, "set_ped_heading", game.set_ped_heading(ped, heading, false));
    }

    /// What the player did in the UI, sent like samp does (chat lines and commands, dialog answers, clicks).
    fn ui_events(&mut self) {
        for o in self.ui.core.take_outgoing() {
            self.note(format!("ui: {o:?}"));
            crate::ui::core::send_outgoing(&mut self.net, o);
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
        let sample = crate::vehicle::DriverSample {
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
        let p = crate::vehicle::build_driver_sync(&sample, self.car_rotation);
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
        self.class_sel.leave();
        report(self, "restore", game.restore());
        report(self, "behind_player", game.behind_player());
        report(self, "set_hud_visible", game.set_hud_visible(true));
        report(self, "set_player_control", game.set_player_control(true));
        report(
            self,
            "request_collision",
            game.request_collision(Vec3::new(s.pos.x, s.pos.y, s.pos.z)),
        );
        self.restart_at(game, Vec3::new(s.pos.x, s.pos.y, s.pos.z), s.rotation);
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
        self.set_skin(game, s.skin as i32);
        self.spawned = true;
        self.life.dead = false;
        self.spawned_once = true;
        // The spawn ends class selection: its mode-0 request restores input only after 10 frames.
        self.ui.spawn_release = true;
        self.sender.reset();
        self.net
            .send_rpc(&player::PlayerSpawn, Reliability::ReliableOrdered);
        self.note(format!(
            "spawned at ({:.2},{:.2},{:.2}) heading {} skin {}",
            s.pos.x, s.pos.y, s.pos.z, s.rotation, s.skin
        ));
    }

    /// A type-8 (debug) chat line, as samp prints its connection lines.
    fn debug_line(&mut self, line: &str) {
        self.note(format!("chat (debug): {line}"));
        self.ui.core.print_debug(line.as_bytes());
    }

    /// A type-4 (info) chat line.
    fn info_line(&mut self, line: &str) {
        self.note(format!("chat (info): {line}"));
        self.ui.core.print_info(line.as_bytes());
    }

    /// The class / spawn skin on the local ped (samp's `CPlayerPed::SetModelIndex`: the adapter loads the
    /// model first). Skin 0 (CJ) needs the clothes rebuild samp does, so it is left alone here.
    fn set_skin(&self, game: &mut dyn GameAdapter, skin: i32) {
        let Some(me) = game.local_player() else { return };
        if skin != 0 && game.is_ped_model(ModelId(skin)) {
            report(self, "set_ped_model", game.set_ped_model(me, ModelId(skin)));
        }
    }

    /// Whether a player id in an RPC names us.
    fn is_me(&self, id: u32) -> bool {
        self.net.player_id().is_some_and(|me| u32::from(me) == id)
    }

    /// Every frame until InitGame, like samp.dll 0x100089B0 (samp-rs `connecting_tick`): weather 1, HUD off, the
    /// ped parked under the camera, the camera fixed above Verona Beach looking at downtown Los Santos.
    fn connecting_scene(&mut self, game: &mut dyn GameAdapter) {
        report(self, "set_weather", game.set_weather(CONNECT_WEATHER));
        report(self, "set_hud_visible", game.set_hud_visible(false));
        if let Some(me) = game.local_player() {
            report(self, "set_ped_position", game.set_ped_position(me, CONNECT_PED_POS));
        }
        report(self, "restore", game.restore());
        report(self, "set_fixed_position", game.set_fixed_position(CONNECT_CAMERA, Vec3::default()));
        report(self, "restore", game.restore());
        report(self, "point_at", game.point_at(CONNECT_LOOK_AT, CameraCut::Cut));
    }

    /// samp's class selection entry (samp-rs `enter_class_selection`): not spawned, health 100, no control.
    fn enter_class_selection(&mut self, game: &mut dyn GameAdapter, count: u32) {
        self.note(format!("entering class selection ({count} class(es))"));
        self.spawned = false;
        self.sender.reset();
        if let Some(me) = game.local_player() {
            report(self, "set_ped_health", game.set_ped_health(me, 100.0));
        }
        report(self, "set_player_control", game.set_player_control(false));
        self.class_sel.enter(Instant::now(), count);
    }

    /// One frame of class selection (samp-rs `class_selection_tick`): HUD off while no menu is open, the panel's
    /// clicks taken, nothing more while a dialog is open (it holds back even the first RPC 128), then the
    /// arrows (previous / next class with samp's sounds) and Shift (spawn request); a panel click acts as its
    /// key (`<<` Left, `>>` Right, `Spawn` Shift).
    fn class_selection_tick(&mut self, game: &mut dyn GameAdapter, now: Instant) {
        if !game.menu_active() {
            report(self, "set_hud_visible", game.set_hud_visible(false));
        }
        let click = self.ui.session.take_class_click();
        if click.is_some() {
            self.note(format!("class panel: {click:?} clicked"));
        }
        if self.ui.core.dialogs.is_open() {
            return;
        }
        // Shift does nothing while the chat input is open (samp-rs `class_selection_keys`).
        let chat_open = self.ui.core.input.is_open();
        let keys = classsel::Keys {
            left: click == Some(ClassButton::Previous) || game.key_held(KEY_LEFT),
            right: click == Some(ClassButton::Next) || game.key_held(KEY_RIGHT),
            shift: click == Some(ClassButton::Spawn)
                || (!chat_open && KEY_SHIFTS.iter().any(|k| game.key_held(*k)))
                || self.auto_spawn,
        };
        match self.class_sel.tick(now, keys) {
            Some(classsel::Action::RequestClass(i)) => {
                self.note(format!("requesting class {i}"));
                self.net
                    .send_rpc(&player::PlayerRequestClass { class_index: i }, Reliability::Reliable);
            }
            Some(classsel::Action::ChangeClass { index, sound }) => {
                if let Some(me) = game.local_player()
                    && let Ok(s) = game.ped_snapshot(me)
                {
                    report(self, "one_off_sound", game.one_off_sound(s.matrix.pos, sound));
                }
                self.note(format!("requesting class {index} (sound {sound})"));
                self.net.send_rpc(
                    &player::PlayerRequestClass { class_index: index },
                    Reliability::Reliable,
                );
            }
            Some(classsel::Action::RequestSpawn) => {
                self.note("requesting spawn".into());
                self.net
                    .send_rpc(&player::PlayerRequestSpawn, Reliability::Reliable);
            }
            None => {}
        }
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

/// The world RPCs of the server's objects (`hostedobjects`).
fn is_object_rpc(w: &samp_proto::rpcs::world::Incoming) -> bool {
    use samp_proto::rpcs::world::Incoming as W;
    matches!(
        w,
        W::CreateObject(_)
            | W::SetObjectPosition(_)
            | W::SetObjectRotation(_)
            | W::DestroyObject(_)
            | W::MoveObject(_)
            | W::StopObject(_)
            | W::AttachObjectToPlayer(_)
            | W::SetPlayerObjectMaterial(_)
            | W::SetObjectNoCameraCollision(_)
            | W::SetPlayerAttachedObject(_)
    )
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
fn report(h: &HostedClient, what: &str, r: sa_samp_api::GameResult<()>) {
    if let Err(e) = r {
        h.note(format!("adapter {what}: {e}"));
    }
}

/// What a nametag shows of another player.
#[derive(Clone, Copy, Debug, Default)]
struct RemoteTag {
    npc: bool,
    health: f32,
    armour: f32,
    synced: Option<Instant>,
}

/// The head's height above a ped's root where the adapter has no bones.
const HEAD_ABOVE_ROOT: f32 = 0.7;

impl HostedClient {
    /// Nametags to draw this frame (docs/re/subsystems/ui-rendering.md, "16.1 Who gets one"): the streamed-in
    /// players that are no NPC, nearer to the camera than the server's distance, anchored at the head (bone 8),
    /// in sight when the server asks for it.
    fn nametags(&self, game: &dyn GameAdapter) -> Vec<TagDraw> {
        let core = &self.ui.core;
        if !core.show_name_tags {
            return Vec::new();
        }
        let cam = game.camera().matrix.pos;
        let mut out = Vec::new();
        for (&id, &ped) in &self.players {
            let tag = self.remotes.get(&id).copied().unwrap_or_default();
            if tag.npc || core.hidden_tags.contains(&id) {
                continue;
            }
            let Ok(s) = game.ped_snapshot(ped) else { continue };
            let p = s.matrix.pos;
            let d = ((p.x - cam.x).powi(2) + (p.y - cam.y).powi(2) + (p.z - cam.z).powi(2)).sqrt();
            if d >= core.name_tag_distance {
                continue;
            }
            let head = game
                .bone_position(ped, BoneId(8), true)
                .unwrap_or_else(|_| Vec3::new(p.x, p.y, p.z + HEAD_ABOVE_ROOT));
            let filter = LineFilter { buildings: true, objects: true, ..LineFilter::default() };
            if core.name_tag_los && !game.line_clear(head, cam, filter) {
                continue;
            }
            let Some(name) = core.name(id) else { continue };
            let mut text = name.to_vec();
            text.extend_from_slice(format!(" ({id})").as_bytes());
            let afk = tag.synced.is_none_or(|t| t.elapsed().as_millis() >= u128::from(samp_ui::nametags::AFK_AFTER_MS));
            out.push(TagDraw {
                anchor: [head.x, head.y, head.z],
                distance: d,
                name: text,
                colour: core.colours.chat_argb(id) | 0xFF00_0000,
                health: tag.health,
                armour: tag.armour,
                afk,
            });
        }
        out
    }
}

impl GameHooks for HostedClient {
    fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {
        if phase == FramePhase::BeforeGameProcess {
            self.tick(game);
        }
        if phase == FramePhase::Present {
            self.ui.world.tags = self.nametags(game);
        }
        self.ui.frame(phase, game);
    }

    fn window_event(&mut self, event: WindowEvent, game: &dyn GameAdapter) -> EventResult {
        self.ui.window_event(event, game)
    }

    fn shutdown(&mut self, _game: &mut dyn GameAdapter) {
        self.net.disconnect(Instant::now());
        self.note("disconnected".into());
    }
}

