//! World objects on the original executable: the original backend's `sa_samp_api::objects` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::objects::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg;

use super::script::backend_op;
use super::{OriginalGame, read_error};
use crate::game;
use crate::patch::Memory;
use crate::script::op;

/// Client-side shorthands for the object calls the client makes with script handles (main thread). A handle that
/// names no object issues nothing (the opcode would fault on it).
#[cfg(windows)]
pub mod calls {
    use sa_samp_api::objects::Objects as _;
    use sa_samp_api::{ObjectRef, ScriptHandle};

    use crate::adapter::original::process;

    fn obj(h: i32) -> Option<ObjectRef> {
        process().object_from_script_handle(ScriptHandle(h))
    }

    pub fn set_visible(h: i32, visible: i32) {
        if let Some(o) = obj(h) {
            let _ = process().set_object_visible(o, visible != 0);
        }
    }

    pub fn set_scale(h: i32, scale: f32) {
        if let Some(o) = obj(h) {
            let _ = process().set_object_scale(o, scale);
        }
    }

    /// `IS_OBJECT_ATTACHED`; a handle that names no object reads as not attached.
    pub fn attached(h: i32) -> bool {
        obj(h).is_some_and(|o| process().object_attached(o).unwrap_or(false))
    }

    pub fn delete(h: i32) {
        if let Some(o) = obj(h) {
            let _ = process().destroy_object(o, false);
        }
    }

    pub fn remove_elegantly(h: i32) {
        if let Some(o) = obj(h) {
            let _ = process().destroy_object(o, true);
        }
    }
}

impl<M: Memory> Objects for OriginalGame<M> {
    /// `CPool::GetRef` on the object pool.
    fn object_script_handle(&self, o: ObjectRef) -> GameResult<ScriptHandle> {
        match game::object_handle(&self.mem, o.0) {
            Ok(Some(h)) => Ok(ScriptHandle(h)),
            Ok(None) => Err(GameError::InvalidHandle),
            Err(e) => Err(read_error(e)),
        }
    }

    /// `CPool::GetAtRef` on the object pool.
    fn object_from_script_handle(&self, h: ScriptHandle) -> Option<ObjectRef> {
        game::object_at_ref(&self.mem, h.0).ok().flatten().map(ObjectRef)
    }

    fn object_matrix(&self, _o: ObjectRef) -> GameResult<Matrix> {
        Err(GameError::Unsupported("Objects::object_matrix"))
    }

    fn object_has_rw(&self, _o: ObjectRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Objects::object_has_rw"))
    }

    /// `IS_OBJECT_ATTACHED` (0685).
    fn object_attached(&self, o: ObjectRef) -> GameResult<bool> {
        let h = self.object_script_handle(o)?.0;
        backend_op(crate::worldents::op::IS_OBJECT_ATTACHED, &[ScriptArg::Int(h)])
    }

    fn object_bounds(&self, _o: ObjectRef) -> GameResult<(Vec3, f32)> {
        Err(GameError::Unsupported("Objects::object_bounds"))
    }

    fn object_ray_test(&self, _o: ObjectRef, _from: Vec3, _to: Vec3) -> GameResult<Option<ObjectHit>> {
        Err(GameError::Unsupported("Objects::object_ray_test"))
    }

    fn create_object(&mut self, _model: ModelId, _pos: Vec3) -> GameResult<ObjectRef> {
        Err(GameError::Unsupported("Objects::create_object"))
    }

    /// `REMOVE_OBJECT_ELEGANTLY` (09A2) when fading, else `DELETE_OBJECT` (0108).
    fn destroy_object(&mut self, o: ObjectRef, fade: bool) -> GameResult<()> {
        let h = self.object_script_handle(o)?.0;
        let op = if fade { op::REMOVE_OBJECT_ELEGANTLY } else { crate::worldents::op::DELETE_OBJECT };
        self.run_op(op, &[ScriptArg::Int(h)]).map(|_| ())
    }

    fn set_object_transform(&mut self, _o: ObjectRef, _pos: Vec3, _rot_deg: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_transform"))
    }

    fn set_object_matrix(&mut self, _o: ObjectRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_matrix"))
    }

    /// `SET_OBJECT_SCALE` (08D2).
    fn set_object_scale(&mut self, o: ObjectRef, scale: f32) -> GameResult<()> {
        let h = self.object_script_handle(o)?.0;
        self.run_op(op::SET_OBJECT_SCALE, &[ScriptArg::Int(h), ScriptArg::Float(scale)]).map(|_| ())
    }

    /// `SET_OBJECT_VISIBLE` (0750).
    fn set_object_visible(&mut self, o: ObjectRef, visible: bool) -> GameResult<()> {
        let h = self.object_script_handle(o)?.0;
        self.run_op(op::SET_OBJECT_VISIBLE, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(visible))]).map(|_| ())
    }

    fn set_object_collision(&mut self, _o: ObjectRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_collision"))
    }

    fn attach_object(&mut self, _o: ObjectRef, _to: AttachTarget, _offset: Vec3, _rot_deg: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::attach_object"))
    }

    fn detach_object(&mut self, _o: ObjectRef) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::detach_object"))
    }

    fn play_object_anim(
        &mut self,
        _o: ObjectRef,
        _anim: &str,
        _block: &str,
        _delta: f32,
        _looped: bool,
    ) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::play_object_anim"))
    }

    fn set_object_material(&mut self, _o: ObjectRef, _index: u8, _m: Option<MaterialOverride>) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_material"))
    }

    fn remove_buildings(&mut self, _model: ModelId, _center: Vec3, _radius: f32) -> GameResult<u32> {
        Err(GameError::Unsupported("Objects::remove_buildings"))
    }

    fn camera_ignore(&mut self, _e: EntityRef) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::camera_ignore"))
    }
}
