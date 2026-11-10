//! Camera. samp-rs today: `behaviourclient/camera.rs`, `camtarget.rs`, `client/*.rs`, `swap.rs`,
//! `combathooks.rs`, `ui/render/world.rs`. Mapping: `docs/samp/game-adapter.md` §4.5.

use crate::{GameResult, Matrix, PedRef, Point2, Vec3, VehicleRef};

/// What the camera looks like this frame (`TheCamera` 0xB6F028).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraState {
    /// Camera matrix (`TheCamera.m_mCameraMatrix`, position at 0xB6F9CC).
    pub matrix: Matrix,
    /// Active cam's front vector (`Cams[0].Front` 0xB6F32C).
    pub front: Vec3,
    /// Active cam's source (`Cams[0].Source` 0xB6F338, also samp's line-of-sight origin).
    pub source: Vec3,
    /// Active cam mode (`Cams[0].Mode` 0xB6F1A8).
    pub mode: u16,
    /// Field of view in degrees (`Cams[0].FOV` 0xB6F250).
    pub fov: f32,
    /// Looking left / right flags (0xB6F1A4 / 0xB6F1A5).
    pub looking_left: bool,
    /// Looking right.
    pub looking_right: bool,
    /// Aspect ratio (`CDraw::ms_fAspectRatio` 0xC3EFA4).
    pub aspect_ratio: f32,
    /// Player weapon mode (0xB6F858).
    pub weapon_mode: u16,
}

/// Interpolation mode for scripted camera moves (`CAMERA_SET_VECTOR_MOVE` 0936, `..._TRACK` 0920).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraCut {
    /// Jump cut.
    Cut,
    /// Smooth interpolation.
    Smooth,
}

/// Camera operations.
pub trait Camera {
    /// Current camera state.
    fn camera(&self) -> CameraState;
    /// Projects a world point to screen pixels (`CSprite::CalcScreenCoors` semantics), `None` behind the
    /// camera. Used for nametags and 3D labels.
    fn world_to_screen(&self, p: Vec3) -> Option<(Point2, f32)>;
    /// Whether a sphere is inside the view frustum (`RwCameraFrustumTestSphere` 0x7EE2D0 on `Scene.camera`
    /// 0xC1703C).
    fn sphere_visible(&self, center: Vec3, radius: f32) -> bool;

    /// `SET_FIXED_CAMERA_POSITION` (015F).
    fn set_fixed_position(&mut self, pos: Vec3, rot: Vec3) -> GameResult<()>;
    /// `POINT_CAMERA_AT_POINT` (0160).
    fn point_at(&mut self, target: Vec3, cut: CameraCut) -> GameResult<()>;
    /// `POINT_CAMERA_AT_CHAR` (0159; ped mode for remote players, see
    /// [`crate::config::Behaviour::point_camera_at_char_ped_mode`]).
    fn point_at_ped(&mut self, ped: PedRef, mode: i32, cut: CameraCut) -> GameResult<()>;
    /// `POINT_CAMERA_AT_CAR` (0158).
    fn point_at_vehicle(&mut self, vehicle: VehicleRef, mode: i32, cut: CameraCut) -> GameResult<()>;
    /// `SET_CAMERA_BEHIND_PLAYER` (0373).
    fn behind_player(&mut self) -> GameResult<()>;
    /// `RESTORE_CAMERA_JUMPCUT` (02EB).
    fn restore(&mut self) -> GameResult<()>;
    /// InterpolateCameraPos/LookAt (`CAMERA_SET_VECTOR_MOVE` 0936 / `CAMERA_SET_VECTOR_TRACK` 0920,
    /// `CAMERA_PERSIST_POS` 0930 / `CAMERA_PERSIST_TRACK` 092F, `CAMERA_RESET_NEW_SCRIPTABLES` 0925).
    fn interpolate(
        &mut self,
        look_at: bool,
        from: Vec3,
        to: Vec3,
        time_ms: i32,
        cut: CameraCut,
    ) -> GameResult<()>;
    /// `SWITCH_WIDESCREEN` (02A3).
    fn set_widescreen(&mut self, on: bool) -> GameResult<()>;
    /// Camera shake (`CCamera::CamShakeNoPos` 0x50A970).
    fn shake(&mut self, strength: f32) -> GameResult<()>;
    /// Overrides the aspect ratio and FOV the player code sees (samp's per-player context; see
    /// [`crate::hooks::PlayerContext`]).
    fn set_fov(&mut self, fov: f32) -> GameResult<()>;
}
