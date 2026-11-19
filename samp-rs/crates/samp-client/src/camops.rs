//! The camera RPCs' steps on the game adapter that the DLL client and the hosted client share
//! (docs/re/subsystems/player-state.md, RPCs 157, 158 and 162).

use sa_samp_api::camera::{Camera, CameraCut};
use sa_samp_api::{GameResult, Vec3};

/// The first error of steps that have all run (the array holds their results).
fn first<const N: usize>(steps: [GameResult<()>; N]) -> GameResult<()> {
    steps.into_iter().collect()
}

/// RPC 157: the scripted moves end, then the camera is fixed at `pos`. Mirrors samp.dll B 0x1009D7B0.
pub fn fixed_position<G: Camera + ?Sized>(game: &mut G, pos: Vec3) -> GameResult<()> {
    first([game.reset_scripted_moves(), game.set_fixed_position(pos, Vec3::default())])
}

/// RPC 158: the scripted moves end, then the camera points at `target`. Mirrors samp.dll B 0x1009D810.
pub fn look_at<G: Camera + ?Sized>(game: &mut G, target: Vec3, cut: CameraCut) -> GameResult<()> {
    first([game.reset_scripted_moves(), game.point_at(target, cut)])
}

/// RPC 162: the kept position and the scripted moves end, then the camera goes behind the player with a jump cut.
pub fn behind_player<G: Camera + ?Sized>(game: &mut G) -> GameResult<()> {
    first([game.persist_position(false), game.reset_scripted_moves(), game.behind_player(), game.restore()])
}
