//! Player markers: the radar blips of the other players, on both hosts.
//!
//! With InitGame's marker mode on, every remote ped the server streams in gets an entity blip (blip only, no
//! marker); a player who is not streamed in shows as a coordinate blip at the position of the last marker sync
//! (208) that had one. The blip colour is the player's colour: the DLL client passes the player id as the colour
//! index (its `GetRadarTraceColour` hook reads the player colour table, so a later RPC 72 recolours it), the hosted
//! client the RGBA value itself (and recolours on RPC 72). A colour with alpha 0 (SetPlayerMarkerForPlayer) draws an
//! invisible blip. Spec: docs/re/subsystems/remote-players-sync.md ("208") and world-entities.md (blips).

use sa_samp_api::world::{BlipTarget, WorldEntities};
use sa_samp_api::{BlipRef, GameResult, PedRef, Vec3};

/// Player ids a 208 record may name (below this).
pub use samp_proto::sync::policy::MAX_BULLET_MARKER_PLAYER_ID_EXCLUSIVE as MAX_PLAYER_ID;
/// `CHANGE_BLIP_DISPLAY` of a ped's marker blip: the radar blip without the marker above the ped.
pub const PED_BLIP_DISPLAY: i32 = 2;

/// `CHANGE_BLIP_SCALE` of a 208 coordinate blip from its colour index: 2 for the player ids, 3 from 1004 on (the
/// fixed colours). Mirrors samp.dll B 0x100A0DA0.
pub fn coord_blip_scale(index: u32) -> i32 {
    if index < u32::from(MAX_PLAYER_ID) { 2 } else { 3 }
}

/// An RGBA colour as a blip colour: `CHANGE_BLIP_COLOUR` reads 0..=8 as the game's own colours, so a black
/// colour with an alpha that low (invisible either way) is passed with alpha 9.
pub fn rgba_blip_colour(rgba: u32) -> u32 {
    if rgba <= 8 { 9 } else { rgba }
}

/// One remote player's markers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerMarker {
    /// The coordinate blip of the last 208 record with a position.
    pub coord_blip: Option<BlipRef>,
    /// The last 208 record had a position.
    pub active: bool,
    /// The blip on the streamed-in ped.
    pub ped_blip: Option<BlipRef>,
}

impl PlayerMarker {
    /// A 208 record: no position removes the coordinate blip; a position replaces it, and a new one is made only
    /// while the player has no ped (not streamed in). Mirrors samp.dll B 0x10009F00.
    pub fn on_record(
        &mut self,
        game: &mut dyn WorldEntities,
        id: u16,
        position: Option<[i16; 3]>,
        streamed: bool,
        colour: u32,
    ) -> GameResult<()> {
        self.remove_coord_blip(game);
        self.active = position.is_some();
        let (Some([x, y, z]), false) = (position, streamed) else { return Ok(()) };
        let pos = Vec3::new(f32::from(x), f32::from(y), f32::from(z));
        let blip = game.add_blip(BlipTarget::Coord { pos, sprite: 0, short_range: true, contact_point: false })?;
        self.coord_blip = Some(blip);
        game.set_blip_colour(blip, colour)?;
        game.set_blip_scale(blip, coord_blip_scale(u32::from(id)))
    }

    /// The ped of a stream-in exists: the coordinate blip goes, and with markers on (`mode` != 0) the ped gets a
    /// blip in the player's colour, display 2.
    pub fn on_spawn(&mut self, game: &mut dyn WorldEntities, mode: u32, ped: PedRef, colour: u32) -> GameResult<()> {
        self.remove_coord_blip(game);
        if mode == 0 {
            return Ok(());
        }
        if let Some(b) = self.ped_blip.take() {
            game.remove_blip(b)?;
        }
        let blip = game.add_blip(BlipTarget::Ped(ped))?;
        self.ped_blip = Some(blip);
        game.set_blip_colour(blip, colour)?;
        game.set_blip_display(blip, PED_BLIP_DISPLAY)
    }

    /// Stream-out of a spawned ped, a quit, a game-mode restart: the coordinate blip goes and the marker is
    /// inactive.
    pub fn reset(&mut self, game: &mut dyn WorldEntities) {
        self.remove_coord_blip(game);
        self.active = false;
    }

    /// The ped was deleted: the game's `CPed` destructor removed its blip.
    pub fn ped_destroyed(&mut self) {
        self.ped_blip = None;
    }

    /// The player's colour changed (hosts that pass the colour itself).
    pub fn recolour(&self, game: &mut dyn WorldEntities, colour: u32) -> GameResult<()> {
        for b in [self.coord_blip, self.ped_blip].into_iter().flatten() {
            game.set_blip_colour(b, colour)?;
        }
        Ok(())
    }

    fn remove_coord_blip(&mut self, game: &mut dyn WorldEntities) {
        if let Some(b) = self.coord_blip.take() {
            // a blip the game already dropped needs nothing more
            let _ = game.remove_blip(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_samp_api::world::{RadarArea, CheckpointKind};
    use sa_samp_api::{CheckpointRef, ModelId, PickupRef};

    #[derive(Default)]
    struct Blips {
        next: u32,
        log: Vec<String>,
    }

    impl WorldEntities for Blips {
        fn create_pickup(&mut self, _: ModelId, _: i32, _: Vec3) -> GameResult<PickupRef> {
            unreachable!()
        }
        fn remove_pickup(&mut self, _: PickupRef) -> GameResult<()> {
            unreachable!()
        }
        fn pickup_collected(&self, _: PickupRef) -> GameResult<bool> {
            unreachable!()
        }
        fn pickup_index(&self, _: PickupRef) -> GameResult<u16> {
            unreachable!()
        }
        fn pickup_touched(&self, _: PickupRef) -> GameResult<bool> {
            unreachable!()
        }
        fn create_checkpoint(&mut self, _: CheckpointKind, _: Vec3, _: f32) -> GameResult<CheckpointRef> {
            unreachable!()
        }
        fn delete_checkpoint(&mut self, _: CheckpointRef) -> GameResult<()> {
            unreachable!()
        }
        fn locate_ped(&mut self, _: PedRef, _: Vec3, _: Vec3, _: bool) -> GameResult<bool> {
            unreachable!()
        }
        fn add_blip(&mut self, t: BlipTarget) -> GameResult<BlipRef> {
            self.next += 1;
            self.log.push(format!("add {} {t:?}", self.next));
            Ok(BlipRef(self.next))
        }
        fn remove_blip(&mut self, b: BlipRef) -> GameResult<()> {
            self.log.push(format!("remove {}", b.0));
            Ok(())
        }
        fn set_blip_colour(&mut self, b: BlipRef, c: u32) -> GameResult<()> {
            self.log.push(format!("colour {} {c:#x}", b.0));
            Ok(())
        }
        fn set_blip_scale(&mut self, b: BlipRef, s: i32) -> GameResult<()> {
            self.log.push(format!("scale {} {s}", b.0));
            Ok(())
        }
        fn set_blip_display(&mut self, b: BlipRef, d: i32) -> GameResult<()> {
            self.log.push(format!("display {} {d}", b.0));
            Ok(())
        }
        fn show_radar_area(&mut self, _: u16, _: &RadarArea) -> GameResult<()> {
            unreachable!()
        }
        fn hide_radar_area(&mut self, _: u16) -> GameResult<()> {
            unreachable!()
        }
    }

    #[test]
    fn far_player_gets_a_coordinate_blip_until_streamed_in() {
        let mut g = Blips::default();
        let mut m = PlayerMarker::default();
        m.on_record(&mut g, 7, Some([100, -200, 10]), false, 7).unwrap();
        assert!(m.active && m.coord_blip == Some(BlipRef(1)));
        m.on_record(&mut g, 7, Some([110, -200, 10]), false, 7).unwrap();
        assert_eq!(m.coord_blip, Some(BlipRef(2)));
        m.on_spawn(&mut g, 1, PedRef(0x500), 7).unwrap();
        assert_eq!((m.coord_blip, m.ped_blip), (None, Some(BlipRef(3))));
        // streamed in: a record with a position makes no coordinate blip
        m.on_record(&mut g, 7, Some([110, -200, 10]), true, 7).unwrap();
        assert_eq!(m.coord_blip, None);
        assert!(g.log.contains(&"scale 1 2".to_string()));
        assert!(g.log.contains(&"display 3 2".to_string()));
        m.on_record(&mut g, 7, None, false, 7).unwrap();
        assert!(!m.active);
    }

    #[test]
    fn markers_off_and_colours() {
        let mut g = Blips::default();
        let mut m = PlayerMarker::default();
        m.on_spawn(&mut g, 0, PedRef(0x500), 7).unwrap();
        assert_eq!(m.ped_blip, None);
        assert_eq!(coord_blip_scale(1003), 2);
        assert_eq!(coord_blip_scale(0xFF00_00FF), 3);
        assert_eq!(rgba_blip_colour(0), 9);
        assert_eq!(rgba_blip_colour(0xFFFF_FF00), 0xFFFF_FF00);
    }
}
