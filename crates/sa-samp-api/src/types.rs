//! Shared value types and entity handles.
//!
//! Handles are opaque 32-bit values. The original and phase A backends store the game object's address
//! (`CPed*`, `CVehicle*`, `CObject*`), which is what samp-rs passes around today. The native backend stores
//! a generational pool reference. samp-rs must treat them as opaque and never do arithmetic on them;
//! [`ScriptHandle`] is the SCM-visible handle (`CPools::GetPedRef` and friends) when a script command needs it.

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
/// `CPlaceable::m_matrix` (+0x14). samp sync packets carry these vectors directly.
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

/// A quaternion (w, x, y, z), the rotation format of samp's sync packets.
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
    /// A particle effect system started by [`crate::world::WorldState::create_fx`] (script FX handle).
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
    /// A ped task created by samp (`CTask*`), only used for the enter/leave task hooks.
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

/// A model index. Signed: samp moves `CModelInfo::ms_modelInfoPtrs` so that −32767…32767 are valid
/// (patch row "model-info pointer array"; the 707 operand sites).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModelId(pub i32);

/// A weapon type (`eWeaponType`, 0..=46).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WeaponType(pub u8);

/// A player slot in `CWorld::Players` (0 = local player; samp's remote players use 2..=209).
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
    /// 7: frontend idle (samp's init point).
    FrontendIdle,
    /// 8: loading started.
    LoadingStarted,
    /// 9: playing.
    Playing,
    /// Any other value.
    Other(i32),
}

/// Frame time steps (1.0 = 1/50 s).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeStep {
    /// `CTimer::ms_fTimeStep` (clipped).
    pub clipped: f32,
    /// `CTimer::ms_fTimeStepNonClipped`.
    pub non_clipped: f32,
}

/// The 30 tracked weapon/vehicle skill stats the context switch and `/weaponskill` read
/// (`CStats::StatTypesFloat + 0x114` 0xB79494, 11 weapon skills used by samp).
pub const WEAPON_SKILL_STATS: usize = 11;
