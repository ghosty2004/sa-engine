//! The camera.

use crate::{Matrix, Vec3};

/// What the camera looks like this frame (`TheCamera` 0xB6F028).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraState {
    /// Camera matrix (`TheCamera.m_mCameraMatrix`, position at 0xB6F9CC).
    pub matrix: Matrix,
    /// Active cam's front vector (`Cams[0].Front` 0xB6F32C).
    pub front: Vec3,
    /// Active cam's source (`Cams[0].Source` 0xB6F338).
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

