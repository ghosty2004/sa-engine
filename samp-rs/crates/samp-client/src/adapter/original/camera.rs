//! The camera on the original executable: the original backend's `sa_samp_api::camera` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::camera::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg;

use super::OriginalGame;
use crate::patch::Memory;
use crate::script::op;

/// Client-side shorthands for the camera calls the client makes through the adapter (main thread).
#[cfg(windows)]
pub mod calls {
    use sa_samp_api::Vec3;
    use sa_samp_api::camera::{Camera as _, CameraCut};

    use crate::adapter::original::process;

    pub fn fixed_position(x: f32, y: f32, z: f32) {
        let _ = process().set_fixed_position(Vec3::new(x, y, z), Vec3::default());
    }

    /// `mode` is the opcode's switch mode: 2 cuts, 1 moves (samp passes nothing else).
    pub fn point_at(x: f32, y: f32, z: f32, mode: i32) {
        let cut = if mode == 2 { CameraCut::Cut } else { CameraCut::Smooth };
        let _ = process().point_at(Vec3::new(x, y, z), cut);
    }

    pub fn behind_player() {
        let _ = process().behind_player();
    }

    pub fn restore() {
        let _ = process().restore();
    }
}

impl<M: Memory> Camera for OriginalGame<M> {
    fn camera(&self) -> CameraState {
        CameraState::default()
    }

    fn world_to_screen(&self, _p: Vec3) -> Option<(Point2, f32)> {
        None
    }

    fn sphere_visible(&self, _center: Vec3, _radius: f32) -> bool {
        false
    }

    /// `SET_FIXED_CAMERA_POSITION` (015F).
    fn set_fixed_position(&mut self, pos: Vec3, rot: Vec3) -> GameResult<()> {
        let a = [pos.x, pos.y, pos.z, rot.x, rot.y, rot.z].map(ScriptArg::Float);
        self.run_op(op::SET_FIXED_CAMERA_POSITION, &a).map(|_| ())
    }

    /// `POINT_CAMERA_AT_POINT` (0160) with switch mode 2 (cut) or 1 (smooth).
    fn point_at(&mut self, target: Vec3, cut: CameraCut) -> GameResult<()> {
        let mode = if cut == CameraCut::Cut { 2 } else { 1 };
        let a =
            [ScriptArg::Float(target.x), ScriptArg::Float(target.y), ScriptArg::Float(target.z), ScriptArg::Int(mode)];
        self.run_op(op::POINT_CAMERA_AT_POINT, &a).map(|_| ())
    }

    fn point_at_ped(&mut self, _ped: PedRef, _mode: i32, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at_ped"))
    }

    fn point_at_vehicle(&mut self, _vehicle: VehicleRef, _mode: i32, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at_vehicle"))
    }

    /// `SET_CAMERA_BEHIND_PLAYER` (0373).
    fn behind_player(&mut self) -> GameResult<()> {
        self.run_op(op::SET_CAMERA_BEHIND_PLAYER, &[]).map(|_| ())
    }

    /// `RESTORE_CAMERA_JUMPCUT` (02EB).
    fn restore(&mut self) -> GameResult<()> {
        self.run_op(op::RESTORE_CAMERA_JUMPCUT, &[]).map(|_| ())
    }

    fn interpolate(
        &mut self,
        _look_at: bool,
        _from: Vec3,
        _to: Vec3,
        _time_ms: i32,
        _cut: CameraCut,
    ) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::interpolate"))
    }

    /// `SWITCH_WIDESCREEN` (02A3).
    fn set_widescreen(&mut self, on: bool) -> GameResult<()> {
        self.run_op(op::SWITCH_WIDESCREEN, &[ScriptArg::Int(i32::from(on))]).map(|_| ())
    }

    fn shake(&mut self, _strength: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::shake"))
    }

    fn set_fov(&mut self, _fov: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::set_fov"))
    }
}
