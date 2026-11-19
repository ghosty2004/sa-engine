//! Mission trains: a locomotive and its carriages placed on the nearest track (`CTrain::CreateMissionTrain`
//! 0x6F7550, through `CREATE_MISSION_TRAIN`), and deleted as a whole (`CTrain::RemoveOneMissionTrain`, through
//! `DELETE_MISSION_TRAIN`). The handlers trust their train type, the type's models, the pools and the vehicle they
//! delete; the checks refuse what they would misuse (`crate::checks`).

use sa_types::script::ScriptArg::{Float as F, Int as I, Local as L};
use sa_types::vehicle::VehicleClass;
use sa_types::{GameError, GameResult, ModelId, Vec3, VehicleRef};

use crate::addr;
use crate::script::op;
use crate::{Game, View};

/// `CTrain`'s train types: one row of 16 model ids per type (zero-terminated), 0x40 bytes each.
const TRAIN_CONFIGS: u32 = 0x8D44F8;
/// The train types (`CREATE_MISSION_TRAIN`'s first argument).
pub const TRAIN_TYPES: u32 = 16;
/// The most carriages of a train type.
const MAX_CARRIAGES: u32 = 16;
/// The tram: its train has no driver.
const TRAM: i32 = 449;
/// `CTrain::m_pPrevCarriage`, `m_pNextCarriage`.
const PREV_CARRIAGE: u32 = 0x5D0;
const NEXT_CARRIAGE: u32 = 0x5D4;

impl<V: View> Game<V> {
    /// The models of train type `kind`, in carriage order.
    pub fn train_models(&self, kind: u32) -> GameResult<Vec<ModelId>> {
        if kind >= TRAIN_TYPES {
            return Err(GameError::Other(format!("no train type {kind}")));
        }
        let row = TRAIN_CONFIGS + kind * 4 * MAX_CARRIAGES;
        Ok((0..MAX_CARRIAGES).map(|i| self.view.read_i32(row + 4 * i)).take_while(|&m| m != 0).map(ModelId).collect())
    }

    /// Why `CreateMissionTrain` cannot build train type `kind` now, if it cannot: a model of the type is not loaded,
    /// or the vehicle pool has no slot for each carriage or the ped pool none for the driver.
    pub(crate) fn train_unbuildable(&self, kind: u32) -> Option<String> {
        let models = match self.train_models(kind) {
            Ok(m) => m,
            Err(e) => return Some(e.to_string()),
        };
        if let Some(m) = models.iter().find(|m| !self.model_loaded(**m)) {
            return Some(format!("model {} of train type {kind} is not loaded", m.0));
        }
        let driver = models.first().is_some_and(|m| m.0 != TRAM);
        let room = addr::VEHICLE_POOL.free_slots(&self.view) >= models.len() as u32
            && (!driver || addr::PED_POOL.free_slots(&self.view) > 0);
        (!room).then(|| "no room in the pools for the train".to_string())
    }

    /// Creates a train of type `kind` on the track nearest to `pos`, its locomotive at `pos` (`CREATE_MISSION_TRAIN`
    /// 06D8), running clockwise or not; its models are loaded first. Returns the first carriage.
    pub fn create_train(&mut self, kind: u32, pos: Vec3, clockwise: bool) -> GameResult<VehicleRef> {
        for m in self.train_models(kind)? {
            self.load_model_now(m)?;
        }
        let h = self.run_op_out(
            op::CREATE_MISSION_TRAIN,
            &[I(kind as i32), F(pos.x), F(pos.y), F(pos.z), I(i32::from(clockwise)), L(0)],
        )?;
        self.vehicle_addr(VehicleRef(h)).map(|_| VehicleRef(h)).map_err(|_| GameError::NotLoaded)
    }

    /// The address of the train carriage `v`, or an error for a vehicle that is no train.
    fn carriage_addr(&self, v: VehicleRef) -> GameResult<u32> {
        let a = self.vehicle_addr(v)?;
        let vt = self.view.read_u32(a + addr::off::VTABLE);
        match addr::VEHICLE_VTABLES.iter().find(|(t, _)| *t == vt) {
            Some((_, VehicleClass::Train)) => Ok(a),
            _ => Err(GameError::Other("not a train".into())),
        }
    }

    /// Whether `v` is the first carriage of a train (what `RemoveOneMissionTrain` deletes whole).
    pub(crate) fn train_head(&self, v: VehicleRef) -> GameResult<()> {
        let a = self.carriage_addr(v)?;
        if self.view.read_u32(a + PREV_CARRIAGE) != 0 {
            return Err(GameError::Other("not the first carriage of its train".into()));
        }
        Ok(())
    }

    /// The carriages of the train whose first carriage is `v`, `v` first, along the carriage links.
    pub fn train_carriages(&self, v: VehicleRef) -> GameResult<Vec<VehicleRef>> {
        let mut a = self.carriage_addr(v)?;
        let mut out = vec![v];
        while out.len() < MAX_CARRIAGES as usize {
            let next = self.view.read_u32(a + NEXT_CARRIAGE);
            let Some(n) = self.vehicle_ref(next) else { break };
            out.push(n);
            a = next;
        }
        Ok(out)
    }

    /// Deletes the train whose first carriage is `v`, with every carriage (`DELETE_MISSION_TRAIN` 07BD).
    pub fn destroy_train(&mut self, v: VehicleRef) -> GameResult<()> {
        self.vehicle_addr(v)?;
        self.run_op(op::DELETE_MISSION_TRAIN, &[I(v.0 as i32)]).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{self, put};

    const TRAIN_VTABLE: u32 = 0x872370;

    #[test]
    fn types_carriages_and_deletion() {
        let mut g = fake::game();
        put(&mut g, TRAIN_CONFIGS + 5 * 0x40, [538i32, 570, 570, 0]);
        assert_eq!(g.train_models(5).unwrap(), vec![ModelId(538), ModelId(570), ModelId(570)]);
        assert!(g.train_models(16).is_err());
        assert!(g.train_unbuildable(5).is_some_and(|w| w.contains("not loaded")));
        let slot = |s: u32| fake::VEHS + s * addr::VEHICLE_POOL.element;
        for s in [3u32, 4] {
            put(&mut g, fake::VEH_MAP + s, 0x01u8);
            put(&mut g, slot(s), TRAIN_VTABLE);
        }
        put(&mut g, slot(3) + NEXT_CARRIAGE, slot(4));
        put(&mut g, slot(4) + PREV_CARRIAGE, slot(3));
        assert_eq!(g.train_carriages(VehicleRef(0x301)).unwrap(), vec![VehicleRef(0x301), VehicleRef(0x401)]);
        assert!(g.destroy_train(VehicleRef(0x401)).is_err());
        assert!(g.destroy_train(VehicleRef(0x301)).is_ok());
        put(&mut g, fake::VEH_MAP + 5, 0x01u8);
        put(&mut g, slot(5), 0x871120u32);
        assert!(g.destroy_train(VehicleRef(0x501)).is_err());
        assert!(g.create_train(16, Vec3::new(0.0, 0.0, 0.0), true).is_err());
    }
}
