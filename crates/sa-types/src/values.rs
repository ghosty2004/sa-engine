//! Value types and entity handles.
//!
//! Handles are opaque 32-bit values. On the ported engine a handle is the generational pool reference the game
//! itself uses for script handles (`CPools::GetPedRef`: `slot << 8 | reuse counter`); a backend inside the original
//! executable may store the object's address instead. Users treat them as opaque; [`ScriptHandle`] is the
//! SCM-visible handle when a script command needs it.

/// A 3D vector (`CVector`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
    /// Z.
    pub z: f32,
}

impl Vec3 {
    /// Builds a vector.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// An entity transform (`CMatrix` without its RW link): right, forward, up and position, as in
/// `CPlaceable::m_matrix` (+0x14).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Matrix {
    /// Right vector (row 0).
    pub right: Vec3,
    /// Forward vector (row 1).
    pub forward: Vec3,
    /// Up vector (row 2).
    pub up: Vec3,
    /// Position (row 3, `CMatrix::pos` +0x30).
    pub pos: Vec3,
}

/// A quaternion (w, x, y, z).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quat {
    /// Scalar part.
    pub w: f32,
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
    /// Z.
    pub z: f32,
}

/// An RGBA colour, one byte per channel (`CRGBA`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha.
    pub a: u8,
}

impl Rgba {
    /// From `0xRRGGBBAA`.
    pub const fn from_rgba_u32(v: u32) -> Self {
        Self {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    }

    /// From D3D's `0xAARRGGBB`.
    pub const fn from_argb_u32(v: u32) -> Self {
        Self {
            a: (v >> 24) as u8,
            r: (v >> 16) as u8,
            g: (v >> 8) as u8,
            b: v as u8,
        }
    }

    /// To D3D's `0xAARRGGBB`.
    pub const fn to_argb_u32(self) -> u32 {
        (self.a as u32) << 24 | (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }
}

/// A point in screen pixels (or in the game's 640x448 HUD space where a method says so).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point2 {
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
}

/// A screen rectangle (`CRect`: left, top, right, bottom).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect2 {
    /// Left.
    pub left: f32,
    /// Top.
    pub top: f32,
    /// Right.
    pub right: f32,
    /// Bottom.
    pub bottom: f32,
}

/// Declares an opaque handle newtype.
macro_rules! handle {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);
    };
}

handle!(
    /// A ped (`CPed*` in the original layout). Local and remote players are peds too.
    PedRef
);
handle!(
    /// A vehicle (`CVehicle*` in the original layout).
    VehicleRef
);
handle!(
    /// A world object (`CObject*` in the original layout).
    ObjectRef
);
handle!(
    /// A pickup (index into `CPickups::aPickUps` 0x9788C0 in the original, plus its generation).
    PickupRef
);
handle!(
    /// A radar blip (script blip handle in the original).
    BlipRef
);
handle!(
    /// A checkpoint or race checkpoint (`CCheckpoint*` / script checkpoint handle in the original).
    CheckpointRef
);
handle!(
    /// A particle effect system (script FX handle).
    FxRef
);
handle!(
    /// A GTA menu (`CMenuSystem` slot; script menu handle in the original).
    MenuRef
);
handle!(
    /// A texture dictionary slot (`CTxdStore` slot index).
    TxdSlot
);
handle!(
    /// A 2D texture owned by the backend (`CSprite2d` / D3D texture in the original, GPU texture natively).
    TextureRef
);
handle!(
    /// A ped task (`CTask*`).
    TaskRef
);

/// Any placeable entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityRef {
    /// A ped.
    Ped(PedRef),
    /// A vehicle.
    Vehicle(VehicleRef),
    /// An object.
    Object(ObjectRef),
    /// A building (static world geometry), identified by its address or native index.
    Building(u32),
    /// A dummy.
    Dummy(u32),
}

/// The handle a script command sees (`CPools::GetPedRef` / `GetVehicleRef` / `GetObjectRef`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScriptHandle(pub i32);

/// A model index (signed: a relocated `CModelInfo::ms_modelInfoPtrs` may make negative indices valid).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModelId(pub i32);

/// A weapon type (`eWeaponType`, 0..=46).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WeaponType(pub u8);

/// A player slot in `CWorld::Players` (0 = the local player).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerSlot(pub u8);

/// An interior / area code (`CGame::currArea` 0xB72914, `CEntity::m_nAreaCode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct AreaCode(pub u8);

/// The game state machine (`gGameState`, `eGameState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameState {
    /// 0..=6: start-up, logos, intro.
    Starting(u8),
    /// 7: frontend idle.
    FrontendIdle,
    /// 8: loading started.
    LoadingStarted,
    /// 9: playing.
    Playing,
    /// Any other value.
    Other(i32),
}

impl Default for GameState {
    /// The value before the game's start-up sets it.
    fn default() -> GameState {
        GameState::Starting(0)
    }
}

impl GameState {
    /// The state of the raw `gGameState` value.
    pub const fn from_raw(v: i32) -> GameState {
        match v {
            0..=6 => GameState::Starting(v as u8),
            7 => GameState::FrontendIdle,
            8 => GameState::LoadingStarted,
            9 => GameState::Playing,
            v => GameState::Other(v),
        }
    }

    /// The raw `gGameState` value.
    pub const fn raw(self) -> i32 {
        match self {
            GameState::Starting(v) => v as i32,
            GameState::FrontendIdle => 7,
            GameState::LoadingStarted => 8,
            GameState::Playing => 9,
            GameState::Other(v) => v,
        }
    }
}

/// Frame time steps (1.0 = 1/50 s).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeStep {
    /// `CTimer::ms_fTimeStep` (clipped).
    pub clipped: f32,
    /// `CTimer::ms_fTimeStepNonClipped`.
    pub non_clipped: f32,
}


/// A feature of the game a host may switch off (the original's own features; all on by default). The mod API's
/// option numbers are the declaration order ([`GameOption::ALL`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameOption {
    /// Typed cheat codes.
    TypedCheats,
    /// The game clock runs and the HUD shows it.
    Clock,
    /// Ambient peds (on foot, as drivers of the cars the game creates, in interiors).
    AmbientPeds,
    /// Random traffic.
    RandomTraffic,
    /// Parked-car generators.
    CarGenerators,
    /// Ambient planes.
    AmbientPlanes,
    /// Cutscenes.
    Cutscenes,
    /// The replay buffer.
    Replay,
    /// Plants and grass.
    Plants,
    /// The sun and moon coronas.
    SunAndMoon,
    /// The idle camera.
    IdleCamera,
    /// Help boxes.
    HelpMessages,
    /// Zone names on entering an area.
    ZoneNames,
    /// The start screens (logos, intro movie, main menu): off, the game loads straight into a new game. Read when
    /// the game starts, so only a mod's `sa_init` or `sa_settings` changes it in time.
    StartScreens,
    /// The vehicle name on the HUD when the player enters a vehicle.
    VehicleNameOnHud,
    /// The vital stats on the HUD while their button is held (off, the radar stays).
    VitalStatsOnHud,
    /// The pickups the map places (weapons, health, armour, interior pickups); read as the map loads, so only a
    /// mod's `sa_init` or `sa_settings` changes the map's own pickups in time.
    SinglePlayerPickups,
    /// Ambient gunfire sounds.
    AmbientGunfire,
    /// Real-time shadows of peds and vehicles.
    RealTimeShadows,
    /// The pause menu opens when the window gets the focus back.
    PauseMenuOnFocus,
    /// The models around the old position are freed when the player respawns.
    DeathModelCleanup,
    /// The speed blur from the player's own speed.
    SpeedBlur,
    /// The "wasted" big message when the player dies.
    WastedMessage,
    /// What the player gets on first driving some vehicles: ambulance health, taxi cash, Enforcer armour, the
    /// Caddy's golf club, Hotdog calories, the police car's shotgun.
    VehicleFreebies,
    /// The player's idle animations while standing still.
    PlayerIdleAnims,
    /// Peds react to a gun aimed at them.
    GunAimedAtResponse,
    /// The sound of firing a sniper rifle or a rocket launcher without aiming it.
    FireFailSounds,
    /// The game changes the player's statistics through its stat setter (scripts, respawns).
    StatChanges,
    /// The handbrake does nothing while the player's controls are disabled.
    HandbrakeNeedsControl,
    /// A ped standing on a vehicle or object takes up a change of speed only gradually (off, at once, as on the
    /// ground).
    StandingSpeedClamp,
}

impl GameOption {
    /// Every option, in declaration order.
    pub const ALL: [GameOption; 30] = [
        GameOption::TypedCheats,
        GameOption::Clock,
        GameOption::AmbientPeds,
        GameOption::RandomTraffic,
        GameOption::CarGenerators,
        GameOption::AmbientPlanes,
        GameOption::Cutscenes,
        GameOption::Replay,
        GameOption::Plants,
        GameOption::SunAndMoon,
        GameOption::IdleCamera,
        GameOption::HelpMessages,
        GameOption::ZoneNames,
        GameOption::StartScreens,
        GameOption::VehicleNameOnHud,
        GameOption::VitalStatsOnHud,
        GameOption::SinglePlayerPickups,
        GameOption::AmbientGunfire,
        GameOption::RealTimeShadows,
        GameOption::PauseMenuOnFocus,
        GameOption::DeathModelCleanup,
        GameOption::SpeedBlur,
        GameOption::WastedMessage,
        GameOption::VehicleFreebies,
        GameOption::PlayerIdleAnims,
        GameOption::GunAimedAtResponse,
        GameOption::FireFailSounds,
        GameOption::StatChanges,
        GameOption::HandbrakeNeedsControl,
        GameOption::StandingSpeedClamp,
    ];
}

/// A limit of the game a host may raise before the game sizes it (all the original's by default). The mod API's
/// limit numbers are the declaration order ([`GameLimit::ALL`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameLimit {
    /// Single-link list nodes (the entity lists of the world sectors), 70000.
    PtrNodesSingle,
    /// Double-link list nodes, 3200.
    PtrNodesDouble,
    /// Entry info nodes (an entity's sector entries), 500.
    EntryInfoNodes,
    /// Peds, 140.
    Peds,
    /// Vehicles, 110.
    Vehicles,
    /// Buildings, 13000.
    Buildings,
    /// Objects, 350.
    Objects,
    /// Dummies, 2500.
    Dummies,
    /// Collision models, 10150.
    ColModels,
    /// Ped tasks, 500.
    Tasks,
    /// Ped events, 200.
    Events,
    /// Ped intelligences (one per ped), 140.
    PedIntelligence,
    /// The matrices entities borrow, 900.
    Matrices,
    /// Texture dictionaries, 5000.
    TextureDictionaries,
    /// The streaming memory budget in bytes, 50 MiB.
    StreamingMemory,
    /// Vehicle structures, one per vehicle model loaded at the same time, 50.
    VehicleStructures,
}

impl GameLimit {
    /// Every limit, in declaration order.
    pub const ALL: [GameLimit; 16] = [
        GameLimit::PtrNodesSingle,
        GameLimit::PtrNodesDouble,
        GameLimit::EntryInfoNodes,
        GameLimit::Peds,
        GameLimit::Vehicles,
        GameLimit::Buildings,
        GameLimit::Objects,
        GameLimit::Dummies,
        GameLimit::ColModels,
        GameLimit::Tasks,
        GameLimit::Events,
        GameLimit::PedIntelligence,
        GameLimit::Matrices,
        GameLimit::TextureDictionaries,
        GameLimit::StreamingMemory,
        GameLimit::VehicleStructures,
    ];
}

/// A file of the game's script a host may provide in place of the install's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptFile {
    /// `data\script\main.scm`: the main script (its MAIN thread creates the player) and the missions.
    Main,
    /// `data\script\script.img`: the streamed scripts the main script declares.
    Streamed,
}

impl ScriptFile {
    /// Every script file, in declaration order.
    pub const ALL: [ScriptFile; 2] = [ScriptFile::Main, ScriptFile::Streamed];

    /// The file's path in the game folder.
    pub const fn path(self) -> &'static str {
        match self {
            ScriptFile::Main => "data\\script\\main.scm",
            ScriptFile::Streamed => "data\\script\\script.img",
        }
    }
}
