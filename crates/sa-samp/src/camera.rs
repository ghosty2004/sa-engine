//! The camera on the native engine: the native backend's `sa_samp_api::camera` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::camera::*;
use sa_samp_api::*;

use crate::addr;
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

    fn set_fixed_position(&mut self, _pos: Vec3, _rot: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::set_fixed_position"))
    }

    fn point_at(&mut self, _target: Vec3, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at"))
    }

    fn point_at_ped(&mut self, _ped: PedRef, _mode: i32, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at_ped"))
    }

    fn point_at_vehicle(&mut self, _vehicle: VehicleRef, _mode: i32, _cut: CameraCut) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::point_at_vehicle"))
    }

    fn behind_player(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::behind_player"))
    }

    fn restore(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::restore"))
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

    fn set_widescreen(&mut self, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::set_widescreen"))
    }

    fn shake(&mut self, _strength: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::shake"))
    }

    fn set_fov(&mut self, _fov: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Camera::set_fov"))
    }
}
