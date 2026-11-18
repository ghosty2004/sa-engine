//! The running game's state, read and written from outside the ports through handles.
//!
//! Everything that drives the game without being part of it (the mod host, the SA:MP game adapter) reads and
//! changes the game through this layer, so there is one implementation of each access.
//!
//! # How the state is read
//!
//! Every port keeps the original's 32-bit layouts (ADR-001): native sa-game is a 32-bit process that maps the
//! legacy address arena at the original addresses, and the wasm32 build keeps the same layouts in linear memory.
//! Globals therefore live at their 1.0 US addresses and objects in the engine's pools with the original field
//! offsets, which is how the ports themselves read them (`GPtr::from_addr`). [`Game`] reads through a [`View`]:
//! [`EngineView`] is the engine's memory, tests use [`fake::FakeView`]. No raw address leaves this crate: handles
//! are generational pool references and every object read goes through a handle that the pool's byte map
//! validates first, so a stale handle is an error instead of a wild read.
//!
//! # Handles
//!
//! [`PedRef`], [`VehicleRef`] and [`ObjectRef`] hold the pool reference the game itself uses for script handles
//! (`CPools::GetPedRef`: `slot << 8 | reuse counter`). A handle stays valid while the slot holds the same object.
//!
//! # How the state is changed
//!
//! Through the game's own code wherever it has a way: one script command run on a private script thread
//! ([`script`]) reaches the ported command handler, which creates entities with model requests, pool allocation and
//! `CWorld::Add`, moves them with the game's setters and deletes them with `CWorld::Remove`. Plain fields that the
//! game itself only stores (health, armour, money) are written directly. Callers decide when: between the game's
//! frames, never inside a port.

pub mod addr;
mod audio;
mod entities;
#[cfg(any(test, feature = "fake"))]
pub mod fake;
pub mod script;
mod view;
mod world;

pub use sa_types::{EntityRef, GameError, GameResult, ObjectRef, PedRef, VehicleRef};
pub use view::{EngineView, Pool, View};

/// The game, through a [`View`] of its memory.
pub struct Game<V: View> {
    /// The memory.
    pub view: V,
    pub(crate) threads: script::Threads,
}

/// The game in this process (sa-game's engine).
pub fn engine() -> Game<EngineView> {
    Game::new(EngineView)
}

impl<V: View> Game<V> {
    /// The game over `view`, its script commands run by the engine's interpreter.
    pub fn new(view: V) -> Self {
        Self::with_executor(view, script::engine_executor)
    }

    /// The game over `view`, its script commands run through `exec` (tests).
    pub fn with_executor(view: V, exec: script::CommandExecutor) -> Self {
        Self { view, threads: script::Threads::new(exec) }
    }

    /// The same game through a boxed view (one type for every view, e.g. the engine's and a test's).
    pub fn into_dyn(self) -> Game<Box<dyn View>>
    where
        V: 'static,
    {
        Game { view: Box::new(self.view), threads: self.threads }
    }

    /// The address of the live entity `e`, or `InvalidHandle`.
    pub fn entity_addr(&self, e: EntityRef) -> GameResult<u32> {
        match e {
            EntityRef::Ped(p) => self.ped_addr(p),
            EntityRef::Vehicle(v) => self.vehicle_addr(v),
            EntityRef::Object(o) => self.object_addr(o),
            _ => Err(GameError::InvalidHandle),
        }
    }

    /// The handle of the pooled ped, vehicle or object that starts at `address` (an argument or result of a game
    /// function), if there is one.
    pub fn entity_at(&self, address: u32) -> Option<EntityRef> {
        if address == 0 {
            return None;
        }
        if let Some(p) = self.ped_ref(address) {
            return Some(EntityRef::Ped(p));
        }
        if let Some(v) = self.vehicle_ref(address) {
            return Some(EntityRef::Vehicle(v));
        }
        addr::OBJECT_POOL.ref_of(&self.view, address).map(|r| EntityRef::Object(ObjectRef(r)))
    }
}
