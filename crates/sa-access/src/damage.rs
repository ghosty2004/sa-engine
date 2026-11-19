//! A ped's last damage, as the game records it for its death: the weapon type and the entity that dealt it.

use sa_types::ped::{DamageSource, PedDamage};
use sa_types::{EntityRef, GameResult, PedRef, WeaponType};

use crate::{Game, View};

/// `CPed::m_nLastWeaponDamage` (the game stores an `eWeaponType` dword; its low byte is the type).
const PED_LAST_WEAPON_DAMAGE: u32 = 0x760;
/// `CPed::m_pLastEntityDamage` (a registered reference: the game clears it when the entity goes).
const PED_LAST_ENTITY_DAMAGE: u32 = 0x764;

impl<V: View> Game<V> {
    /// The ped's last damage: the weapon type and its source, a pooled ped or vehicle by its handle.
    pub fn ped_damage(&self, ped: PedRef) -> GameResult<PedDamage> {
        let p = self.ped_addr(ped)?;
        let weapon = WeaponType(self.view.read_u8(p + PED_LAST_WEAPON_DAMAGE));
        let source = match self.view.read_u32(p + PED_LAST_ENTITY_DAMAGE) {
            0 => DamageSource::None,
            e => match self.entity_at(e) {
                Some(EntityRef::Ped(d)) => DamageSource::Ped(d),
                Some(EntityRef::Vehicle(v)) => DamageSource::Vehicle(v),
                _ => DamageSource::Other,
            },
        };
        Ok(PedDamage { weapon, source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{self, put};

    #[test]
    fn weapon_and_source() {
        let mut g = fake::game();
        let ped = fake::LOCAL;
        put(&mut g, ped + PED_LAST_WEAPON_DAMAGE, 51u32);
        assert_eq!(g.ped_damage(PedRef(0x205)), Ok(PedDamage { weapon: WeaponType(51), source: DamageSource::None }));
        put(&mut g, fake::VEH_MAP + 3, 0x01u8);
        put(&mut g, ped + PED_LAST_ENTITY_DAMAGE, fake::VEHS + 3 * crate::addr::VEHICLE_POOL.element);
        assert_eq!(g.ped_damage(PedRef(0x205)).map(|d| d.source), Ok(DamageSource::Vehicle(sa_types::VehicleRef(0x301))));
        put(&mut g, ped + PED_LAST_ENTITY_DAMAGE, 0x0123_4560u32);
        assert_eq!(g.ped_damage(PedRef(0x205)).map(|d| d.source), Ok(DamageSource::Other));
        assert!(g.ped_damage(PedRef(0x301)).is_err());
    }
}
