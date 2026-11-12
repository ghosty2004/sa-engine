//! The camera on the native engine: the native backend's `sa_samp_api::camera` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::camera::*;
use sa_samp_api::script::ScriptArg::{Float as F, Int as I};
use sa_samp_api::*;

use crate::addr;
use crate::script::op;
use crate::{NativeGame, View};

impl<V: View> Camera for NativeGame<V> {
    fn camera(&self) -> CameraState {
        use addr::camera as c;
        let v = &self.view;
        CameraState {
            matrix: v.read_matrix(c::MATRIX),
            front: v.read_vec3(c::FRONT),
            source: v.read_vec3(c::SOURCE),
            mode: v.read_u16(c::MODE),
            fov: v.read_f32(c::FOV),
            looking_left: v.read_u8(c::LOOKING_LEFT) != 0,
            looking_right: v.read_u8(c::LOOKING_RIGHT) != 0,
            aspect_ratio: v.read_f32(c::ASPECT_RATIO),
            weapon_mode: v.read_u16(c::WEAPON_MODE),
        }
    }

    fn world_to_screen(&self, _p: Vec3) -> Option<(Point2, f32)> {
        None
    }

    fn sphere_visible(&self, _center: Vec3, _radius: f32) -> bool {
        false
    }

    fn set_fixed_position(&mut self, pos: Vec3, rot: Vec3) -> GameResult<()> {
        self.run_op(
            op::SET_FIXED_CAMERA_POSITION,
            &[F(pos.x), F(pos.y), F(pos.z), F(rot.x), F(rot.y), F(rot.z)],
        )
        .map(drop)
    }

    fn point_at(&mut self, target: Vec3, cut: CameraCut) -> GameResult<()> {
        let mode = match cut {
            CameraCut::Cut => 2,
            CameraCut::Smooth => 1,
        };
        self.run_op(
            op::POINT_CAMERA_AT_POINT,
            &[F(target.x), F(target.y), F(target.z), I(mode)],
        )
        .map(drop)
    }

    fn point_at_ped(&mut self, _ped: PedRef, _mode: i32, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at_ped"))
    }

    fn point_at_vehicle(&mut self, _vehicle: VehicleRef, _mode: i32, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at_vehicle"))
    }

    fn behind_player(&mut self) -> GameResult<()> {
        self.run_op(op::SET_CAMERA_BEHIND_PLAYER, &[]).map(drop)
    }

    fn restore(&mut self) -> GameResult<()> {
        self.run_op(op::RESTORE_CAMERA_JUMPCUT, &[]).map(drop)
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

    fn set_widescreen(&mut self, on: bool) -> GameResult<()> {
        self.run_op(op::SWITCH_WIDESCREEN, &[I(i32::from(on))]).map(drop)
    }

    fn shake(&mut self, _strength: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::shake"))
    }

    fn set_fov(&mut self, _fov: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::set_fov"))
    }
}
