//! World objects on the native engine: the native backend's `sa_samp_api::objects` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::objects::*;
use sa_samp_api::script::ScriptArg::{Float as F, Int as I, Local as L};
use sa_samp_api::*;

use crate::addr::{self, off};
use crate::script::op;
use crate::{NativeGame, View};

impl<V: View> NativeGame<V> {
    /// The address of a live object, or `InvalidHandle`.
    pub(crate) fn object_addr(&self, o: ObjectRef) -> GameResult<u32> {
        addr::OBJECT_POOL
            .at_ref(&self.view, o.0)
            .ok_or(GameError::InvalidHandle)
    }
}

impl<V: View> Objects for NativeGame<V> {
    fn object_script_handle(&self, o: ObjectRef) -> GameResult<ScriptHandle> {
        self.object_addr(o).map(|_| ScriptHandle(o.0 as i32))
    }

    fn object_from_script_handle(&self, h: ScriptHandle) -> Option<ObjectRef> {
        addr::OBJECT_POOL
            .at_ref(&self.view, h.0 as u32)
            .map(|_| ObjectRef(h.0 as u32))
    }

    fn object_matrix(&self, o: ObjectRef) -> GameResult<Matrix> {
        self.object_addr(o).map(|p| self.entity_matrix(p))
    }

    fn object_has_rw(&self, o: ObjectRef) -> GameResult<bool> {
        self.object_addr(o)
            .map(|p| self.view.read_u32(p + off::RW_OBJECT) != 0)
    }

    fn object_attached(&self, _o: ObjectRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Objects::object_attached"))
    }

    fn object_bounds(&self, _o: ObjectRef) -> GameResult<(Vec3, f32)> {
        Err(GameError::Unsupported("Objects::object_bounds"))
    }

    fn object_ray_test(&self, _o: ObjectRef, _from: Vec3, _to: Vec3) -> GameResult<Option<ObjectHit>> {
        Err(GameError::Unsupported("Objects::object_ray_test"))
    }

    fn create_object(&mut self, model: ModelId, pos: Vec3) -> GameResult<ObjectRef> {
        let h = self.run_op_out(
            op::CREATE_OBJECT,
            &[I(model.0), F(pos.x), F(pos.y), F(pos.z), L(0)],
        )?;
        self.object_from_script_handle(ScriptHandle(h as i32))
            .ok_or(GameError::NotLoaded)
    }

    fn destroy_object(&mut self, o: ObjectRef, _fade: bool) -> GameResult<()> {
        let h = self.object_script_handle(o)?.0;
        self.run_op(op::DELETE_OBJECT, &[I(h)]).map(drop)
    }

    fn set_object_transform(&mut self, o: ObjectRef, pos: Vec3, rot_deg: Vec3) -> GameResult<()> {
        let h = self.object_script_handle(o)?.0;
        self.run_op(op::SET_OBJECT_COORDINATES, &[I(h), F(pos.x), F(pos.y), F(pos.z)])?;
        self.run_op(
            op::SET_OBJECT_ROTATION,
            &[I(h), F(rot_deg.x), F(rot_deg.y), F(rot_deg.z)],
        )
        .map(drop)
    }

    fn set_object_matrix(&mut self, _o: ObjectRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_matrix"))
    }

    fn set_object_scale(&mut self, _o: ObjectRef, _scale: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_scale"))
    }

    fn set_object_visible(&mut self, _o: ObjectRef, _visible: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_visible"))
    }

    fn set_object_collision(&mut self, _o: ObjectRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_collision"))
    }

    fn attach_object(
        &mut self,
        _o: ObjectRef,
        _to: AttachTarget,
        _offset: Vec3,
        _rot_deg: Vec3,
    ) -> GameResult<()> {
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

    fn set_object_material(
        &mut self,
        _o: ObjectRef,
        _index: u8,
        _m: Option<MaterialOverride>,
    ) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::set_object_material"))
    }

    fn remove_buildings(&mut self, _model: ModelId, _center: Vec3, _radius: f32) -> GameResult<u32> {
        Err(GameError::Unsupported("Objects::remove_buildings"))
    }

    fn camera_ignore(&mut self, _e: EntityRef) -> GameResult<()> {
        Err(GameError::Unsupported("Objects::camera_ignore"))
    }
}
