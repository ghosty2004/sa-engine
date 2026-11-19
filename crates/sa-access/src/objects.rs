//! Objects: their state and transform, attachment, scale, visibility and collision, materials drawn in place of
//! their own, model queries, ped bone transforms, and buildings removed from the world.

use std::cell::RefCell;
use std::collections::HashMap;

use sa_core::mem::GPtr;
use sa_types::script::ScriptArg::{Float as F, Int as I};
use sa_types::{
    EntityRef, GameError, GameResult, Matrix, ModelId, ModelKind, ObjectRef, ObjectSnapshot, PedRef, Vec3,
};

use crate::addr::{self, matrix, off};
use crate::control::c_str;
use crate::script::op;
use crate::{EngineView, Game, View};

sa_core::orig! {
    /// `CEntity::Teleport` (vtable slot 0x38; `CObject::Teleport`: out of the world, the new position, the RW
    /// matrix and frame, back into the world).
    fn entity_teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, reset_rotation: u32) = thiscall vslot 0x38 => unported;
}

/// `CEntity` flag dword (+0x1C): `bUsesCollision` bit 0, `bIsVisible` bit 7.
const ENTITY_FLAGS: u32 = 0x1C;
const FLAG_USES_COLLISION: u32 = 1;
const FLAG_VISIBLE: u32 = 0x80;
/// `CPhysical::m_pAttachedTo`.
const ATTACHED_TO: u32 = 0xFC;
/// `CObject::m_fScale`.
const OBJECT_SCALE: u32 = 0x15C;
/// `CBaseModelInfo::m_pColModel` and `CColModel::m_boundSphere` (centre +0x18, radius +0x24).
const MI_COL_MODEL: u32 = 0x14;
const COL_SPHERE_CENTRE: u32 = 0x18;
const COL_SPHERE_RADIUS: u32 = 0x24;
/// `CBaseModelInfo::m_fDrawDistance`.
const MI_DRAW_DISTANCE: u32 = 0x18;
/// `CPed::m_apBones` (`AnimBlendFrameData*` per `ePedNode`; the bone id at +0x14).
const PED_BONES: u32 = 0x488;
const FRAME_DATA_BONE_ID: u32 = 0x14;
/// `RpHAnimHierarchy::pMatrixArray`, 0x40 bytes per node.
const HANIM_MATRICES: u32 = 0x8;
/// `ePedNode`: 0 unknown, 1 upper torso ... 18 right foot.
pub const PED_NODES: std::ops::RangeInclusive<u8> = 1..=18;

/// `CPools::ms_pBuildingPool` and `ms_pDummyPool` (slot `sizeof(CBuilding)` / `sizeof(CDummy)`).
const BUILDING_POOL: crate::Pool = crate::Pool { global: 0xB74498, element: 0x38 };
const DUMMY_POOL: crate::Pool = crate::Pool { global: 0xB744A0, element: 0x38 };
/// `COcclusion::aOccluders` (0x12-byte records: centre and size as `i16 / 4`, then rotations and the list link at
/// +0x10) and `COcclusion::NumOccluders`.
const OCCLUDERS: u32 = 0xC73FA0;
const OCCLUDER_COUNT: u32 = 0xC73F98;
const OCCLUDER_SIZE: u32 = 0x12;
const OCCLUDER_MAX: i32 = 1000;
/// The occluder bytes a removal clears: centre and size. The rotations and the link stay, the game walks its
/// occluder lists through the link every frame.
const OCCLUDER_CLEARED: std::ops::Range<u32> = 0..0xC;
/// Occluders within this distance of a removal's centre stop hiding anything.
pub const REMOVAL_OCCLUDER_RANGE: f32 = 500.0;
/// How far a removed entity sinks below the world.
pub const REMOVAL_SINK: f32 = 2000.0;
/// The area a removed entity is moved to (an interior: never drawn outside).
const REMOVAL_AREA: u8 = 1;

/// Model info vtables: the atomic kinds, clump, weapon, ped and vehicle.
const VTBL_ATOMIC_KINDS: [u32; 5] = [0x85BBF0, 0x85BC30, 0x85BC70, 0x85BCB0, 0x85BCF0];
const VTBL_CLUMP_MODEL_INFO: u32 = 0x85BD30;
const VTBL_WEAPON_MODEL_INFO: u32 = 0x85BD78;
pub(crate) const VTBL_VEHICLE_MODEL_INFO: u32 = 0x85C5C8;

/// Material slots of an object that can be drawn with another texture or colour.
pub const MATERIAL_SLOTS: usize = 16;

/// A building removal: entities of `model` (-1: any) whose position is within `radius` of `centre`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Removal {
    /// Model, -1 for any.
    pub model: i32,
    /// Centre.
    pub centre: Vec3,
    /// Radius.
    pub radius: f32,
}

impl Removal {
    /// Whether it removes an entity of `model` at `pos`.
    pub fn matches(&self, model: i32, pos: Vec3) -> bool {
        (self.model == -1 || self.model == model) && dist(self.centre, pos) <= self.radius
    }
}

fn dist(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// A material slot drawn differently: a texture (`RwTexture*`, a reference the slot holds) and a colour
/// (`0xRRGGBBAA`, 0: the material's own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaterialSlot {
    /// The texture, 0 for the material's own.
    pub texture: u32,
    /// The colour, 0 for the material's own.
    pub colour: u32,
}

thread_local! {
    /// The removals of this session, applied to the world's entities as they load ([`apply_removals`]).
    static REMOVALS: RefCell<Vec<Removal>> = const { RefCell::new(Vec::new()) };
    /// Entities the world loaded after a removal and [`apply_removals`] removed.
    static REMOVED_ON_LOAD: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// The objects drawn with other materials, by handle ([`materials_before_render`]).
    static MATERIALS: RefCell<HashMap<u32, [MaterialSlot; MATERIAL_SLOTS]>> = RefCell::new(HashMap::new());
    /// Per object with materials: its RenderWare object and how often it was drawn with its own materials.
    static OWN_DRAWS: RefCell<HashMap<u32, (u32, u8)>> = RefCell::new(HashMap::new());
}

impl<V: View> Game<V> {
    /// The object's state.
    pub fn object_snapshot(&self, o: ObjectRef) -> GameResult<ObjectSnapshot> {
        let p = self.object_addr(o)?;
        let v = &self.view;
        let matrix = self.entity_matrix(p);
        let model = ModelId(i32::from(v.read_i16(p + off::MODEL)));
        let flags = v.read_u32(p + ENTITY_FLAGS);
        let col = self.model_info(model).map_or(0, |mi| v.read_u32(mi + MI_COL_MODEL));
        let (bound_centre, bound_radius) = if col != 0 {
            (transform(&matrix, v.read_vec3(col + COL_SPHERE_CENTRE)), v.read_f32(col + COL_SPHERE_RADIUS))
        } else {
            (matrix.pos, 0.0)
        };
        Ok(ObjectSnapshot {
            matrix,
            model,
            area: sa_types::AreaCode(v.read_u8(p + off::AREA)),
            has_rw: v.read_u32(p + off::RW_OBJECT) != 0,
            visible: flags & FLAG_VISIBLE != 0,
            collision: flags & FLAG_USES_COLLISION != 0,
            attached: v.read_u32(p + ATTACHED_TO) != 0,
            scale: v.read_f32(p + OBJECT_SCALE),
            move_speed: v.read_vec3(p + off::MOVE_SPEED),
            turn_speed: v.read_vec3(p + off::TURN_SPEED),
            bound_centre,
            bound_radius,
        })
    }

    /// Moves the object to `pos` as the game teleports one (`CObject::Teleport`: out of the world, the new
    /// position, the RenderWare matrix and frame, back in), keeping its rotation.
    pub fn teleport_object(&mut self, o: ObjectRef, pos: Vec3) -> GameResult<()> {
        let p = self.object_addr(o)?;
        if !(pos.x.is_finite() && pos.y.is_finite() && pos.z.is_finite()) {
            return Err(GameError::Other("position is not finite".into()));
        }
        entity_teleport(GPtr::from_addr(p as usize), pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits(), 0);
        Ok(())
    }

    /// Gives the object the transform `m` (rows as they are, so a scaled matrix scales it): the rows go into its
    /// matrix, then [`Game::teleport_object`] to the position.
    pub fn set_object_matrix(&mut self, o: ObjectRef, m: &Matrix) -> GameResult<()> {
        let p = self.object_addr(o)?;
        let rows = [m.right, m.forward, m.up, m.pos];
        if !rows.iter().all(|r| r.x.is_finite() && r.y.is_finite() && r.z.is_finite()) {
            return Err(GameError::Other("matrix is not finite".into()));
        }
        let mat = self.view.read_u32(p + off::MATRIX);
        if mat == 0 {
            return Err(GameError::Other("the object has no matrix".into()));
        }
        for (row, at) in [(m.right, matrix::RIGHT), (m.forward, matrix::FORWARD), (m.up, matrix::UP)] {
            for (i, c) in [row.x, row.y, row.z].into_iter().enumerate() {
                self.view.write(mat + at + 4 * i as u32, &c.to_le_bytes());
            }
        }
        self.teleport_object(o, m.pos)
    }

    /// Turns the object to Euler angles in degrees (`SET_OBJECT_ROTATION` 0453).
    pub fn set_object_rotation(&mut self, o: ObjectRef, deg: Vec3) -> GameResult<()> {
        self.object_addr(o)?;
        self.run_op(op::SET_OBJECT_ROTATION, &[I(o.0 as i32), F(deg.x), F(deg.y), F(deg.z)]).map(drop)
    }

    /// Scales the object's drawing (`SET_OBJECT_SCALE` 08D2).
    pub fn set_object_scale(&mut self, o: ObjectRef, scale: f32) -> GameResult<()> {
        self.object_addr(o)?;
        self.run_op(op::SET_OBJECT_SCALE, &[I(o.0 as i32), F(scale)]).map(drop)
    }

    /// Shows or hides the object (`SET_OBJECT_VISIBLE` 0750).
    pub fn set_object_visible(&mut self, o: ObjectRef, on: bool) -> GameResult<()> {
        self.object_addr(o)?;
        self.run_op(op::SET_OBJECT_VISIBLE, &[I(o.0 as i32), I(i32::from(on))]).map(drop)
    }

    /// Switches the object's collision (`SET_OBJECT_COLLISION` 0382).
    pub fn set_object_collision(&mut self, o: ObjectRef, on: bool) -> GameResult<()> {
        self.object_addr(o)?;
        self.run_op(op::SET_OBJECT_COLLISION, &[I(o.0 as i32), I(i32::from(on))]).map(drop)
    }

    /// Writes the object's move and turn speed (`CPhysical` +0x44 / +0x50, units per 1/50 s): what peds standing
    /// on it move with.
    pub fn set_object_speed(&mut self, o: ObjectRef, move_speed: Vec3, turn_speed: Vec3) -> GameResult<()> {
        self.set_physical_speed(EntityRef::Object(o), move_speed, Some(turn_speed))
    }

    /// Attaches the object to a ped, vehicle or object with an offset and a rotation in degrees
    /// (`ATTACH_OBJECT_TO_CHAR` 069B, `ATTACH_OBJECT_TO_CAR` 0681, `ATTACH_OBJECT_TO_OBJECT` 069A).
    pub fn attach_object(&mut self, o: ObjectRef, to: EntityRef, offset: Vec3, rot_deg: Vec3) -> GameResult<()> {
        self.object_addr(o)?;
        self.entity_addr(to)?;
        let (opcode, target) = match to {
            EntityRef::Ped(p) => (op::ATTACH_OBJECT_TO_CHAR, p.0),
            EntityRef::Vehicle(v) => (op::ATTACH_OBJECT_TO_CAR, v.0),
            EntityRef::Object(t) if t != o => (op::ATTACH_OBJECT_TO_OBJECT, t.0),
            _ => return Err(GameError::InvalidHandle),
        };
        let args = [
            I(o.0 as i32),
            I(target as i32),
            F(offset.x),
            F(offset.y),
            F(offset.z),
            F(rot_deg.x),
            F(rot_deg.y),
            F(rot_deg.z),
        ];
        self.run_op(opcode, &args).map(drop)
    }

    /// Detaches the object where it is, with collision (`DETACH_OBJECT` 0682).
    pub fn detach_object(&mut self, o: ObjectRef) -> GameResult<()> {
        self.object_addr(o)?;
        self.run_op(op::DETACH_OBJECT, &[I(o.0 as i32), F(0.0), F(0.0), F(0.0), I(1)]).map(drop)
    }

    /// The model info of `model` (`CModelInfo::ms_modelInfoPtrs`), if it has one.
    fn model_info(&self, model: ModelId) -> Option<u32> {
        let m = u32::try_from(model.0).ok().filter(|&m| m < addr::MODEL_INFO_COUNT)?;
        Some(self.view.read_u32(addr::MODEL_INFO_PTRS + 4 * m)).filter(|&mi| mi != 0)
    }

    /// What `model` is.
    pub fn model_kind(&self, model: ModelId) -> ModelKind {
        let Some(mi) = self.model_info(model) else { return ModelKind::None };
        match self.view.read_u32(mi) {
            vt if VTBL_ATOMIC_KINDS.contains(&vt) => ModelKind::Atomic,
            VTBL_CLUMP_MODEL_INFO => ModelKind::Clump,
            VTBL_WEAPON_MODEL_INFO => ModelKind::Weapon,
            addr::VTBL_PED_MODEL_INFO => ModelKind::Ped,
            VTBL_VEHICLE_MODEL_INFO => ModelKind::Vehicle,
            _ => ModelKind::None,
        }
    }

    /// Sets the draw distance of an atomic model (`CBaseModelInfo::m_fDrawDistance`, the distance the renderer
    /// draws its instances up to); returns the old one.
    pub fn set_model_draw_distance(&mut self, model: ModelId, distance: f32) -> GameResult<f32> {
        if self.model_kind(model) != ModelKind::Atomic || !distance.is_finite() || distance <= 0.0 {
            return Err(GameError::Other(format!("model {} takes no draw distance {distance}", model.0)));
        }
        let mi = self.model_info(model).ok_or(GameError::NotLoaded)?;
        let old = self.view.read_f32(mi + MI_DRAW_DISTANCE);
        self.view.write(mi + MI_DRAW_DISTANCE, &distance.to_le_bytes());
        Ok(old)
    }

    /// The world transform of ped node `node` ([`PED_NODES`], `ePedNode`) of `ped`, from its skin's animation
    /// hierarchy brought up to date first (`CEntity::UpdateRpHAnim`).
    pub fn ped_bone_matrix(&self, ped: PedRef, node: u8) -> GameResult<Matrix> {
        let p = self.ped_addr(ped)?;
        if !PED_NODES.contains(&node) {
            return Err(GameError::Other(format!("no ped node {node}")));
        }
        let clump = self.view.read_u32(p + off::RW_OBJECT);
        let frame_data = self.view.read_u32(p + PED_BONES + 4 * u32::from(node));
        if clump == 0 || frame_data == 0 {
            return Err(GameError::NotLoaded);
        }
        let bone_id = self.view.read_u32(frame_data + FRAME_DATA_BONE_ID);
        let ped_ptr = GPtr::<u8>::from_addr(p as usize);
        sa_core::bindings::entity_update_rp_hanim(ped_ptr);
        let hier = sa_core::bindings::get_anim_hierarchy_from_skin_clump(GPtr::from_addr(clump as usize));
        if hier.is_null() {
            return Err(GameError::NotLoaded);
        }
        let index = sa_core::bindings::rp_hanim_id_get_index(hier, bone_id);
        let matrices = self.view.read_u32(hier.addr() as u32 + HANIM_MATRICES);
        if index < 0 || matrices == 0 {
            return Err(GameError::NotLoaded);
        }
        Ok(self.view.read_matrix(matrices + 0x40 * index as u32))
    }

    /// Removes the buildings, dummies and objects of `model` (-1: any) whose position is within `radius` of
    /// `centre`: they move to an interior area and sink [`REMOVAL_SINK`] below the world, the occluders within
    /// [`REMOVAL_OCCLUDER_RANGE`] stop hiding anything, and the entities the world loads later get the same
    /// ([`apply_removals`]). Returns the entities removed now.
    pub fn remove_buildings(&mut self, model: i32, centre: Vec3, radius: f32) -> GameResult<u32> {
        if !(centre.x.is_finite() && centre.y.is_finite() && centre.z.is_finite() && radius.is_finite()) {
            return Err(GameError::Other("removal centre or radius is not finite".into()));
        }
        let r = Removal { model, centre, radius };
        REMOVALS.with_borrow_mut(|l| l.push(r));
        let v = &mut self.view;
        let n = v.read_i32(OCCLUDER_COUNT).clamp(0, OCCLUDER_MAX) as u32;
        for i in 0..n {
            let o = OCCLUDERS + OCCLUDER_SIZE * i;
            let c = Vec3::new(
                f32::from(v.read_i16(o)) / 4.0,
                f32::from(v.read_i16(o + 2)) / 4.0,
                f32::from(v.read_i16(o + 4)) / 4.0,
            );
            if dist(c, centre) < REMOVAL_OCCLUDER_RANGE {
                for b in OCCLUDER_CLEARED {
                    v.write(o + b, &[0]);
                }
            }
        }
        let mut removed = 0;
        for pool in [BUILDING_POOL, DUMMY_POOL, addr::OBJECT_POOL] {
            for h in pool.refs(&*v) {
                if let Some(e) = pool.at_ref(&*v, h)
                    && sink_if(v, e, &[r])
                {
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    /// Draws material `slot` of the object with `texture` (`RwTexture*` from [`Game::model_texture`], held by the
    /// slot from now on; 0: the material's own) and `colour` (`0xRRGGBBAA`, 0: the material's own); both 0 clears
    /// the slot.
    pub fn set_object_material(&mut self, o: ObjectRef, slot: usize, texture: u32, colour: u32) -> GameResult<()> {
        self.object_addr(o)?;
        if slot >= MATERIAL_SLOTS {
            return Err(GameError::Other(format!("no material slot {slot}")));
        }
        MATERIALS.with_borrow_mut(|m| {
            let slots = m.entry(o.0).or_default();
            release_texture(slots[slot].texture);
            slots[slot] = MaterialSlot { texture, colour };
            if slots.iter().all(|s| *s == MaterialSlot::default()) {
                m.remove(&o.0);
                OWN_DRAWS.with_borrow_mut(|d| d.remove(&o.0));
            }
        });
        Ok(())
    }

    /// Forgets the material slots of an object (it is gone), releasing their textures.
    pub fn clear_object_materials(&mut self, o: ObjectRef) {
        OWN_DRAWS.with_borrow_mut(|d| d.remove(&o.0));
        if let Some(slots) = MATERIALS.with_borrow_mut(|m| m.remove(&o.0)) {
            slots.iter().for_each(|s| release_texture(s.texture));
        }
    }

    /// The texture `name` of the texture dictionary `dict` that model `model` uses, the model loaded first
    /// (`RwTextureRead` with the dictionary current: a new reference for the caller). `NotLoaded` when the
    /// dictionary or the texture is not there.
    pub fn model_texture(&mut self, model: ModelId, dict: &[u8], name: &[u8]) -> GameResult<u32> {
        self.load_model_now(model)?;
        let mut d = c_str(dict);
        let mut n = c_str(name);
        let slot = sa_core::bindings::find_txd_slot(GPtr::from_raw(d.as_mut_ptr()));
        if slot < 0 {
            return Err(GameError::NotLoaded);
        }
        sa_core::bindings::push_current_txd();
        sa_core::bindings::set_current_txd(slot);
        let t = sa_core::bindings::rw_texture_read(GPtr::from_raw(n.as_mut_ptr()), GPtr::NULL);
        sa_core::bindings::pop_current_txd();
        if t.is_null() { Err(GameError::NotLoaded) } else { Ok(t.addr() as u32) }
    }
}

fn release_texture(t: u32) {
    if t != 0 {
        sa_core::bindings::rw_texture_destroy(GPtr::from_addr(t as usize));
    }
}

/// `p` transformed by `m`.
fn transform(m: &Matrix, p: Vec3) -> Vec3 {
    let c = |i: fn(Vec3) -> f32| i(m.right) * p.x + i(m.forward) * p.y + i(m.up) * p.z + i(m.pos);
    Vec3::new(c(|v| v.x), c(|v| v.y), c(|v| v.z))
}

/// Sinks the entity at `e` when a removal of `list` matches it: area [`REMOVAL_AREA`], placement and matrix z
/// lowered by [`REMOVAL_SINK`].
fn sink_if<V: View + ?Sized>(v: &mut V, e: u32, list: &[Removal]) -> bool {
    let model = i32::from(v.read_u16(e + off::MODEL));
    let m = v.read_u32(e + off::MATRIX);
    let pos = if m != 0 { v.read_vec3(m + matrix::POS) } else { v.read_vec3(e + off::PLACEMENT_POS) };
    if !list.iter().any(|r| r.matches(model, pos)) {
        return false;
    }
    v.write(e + off::AREA, &[REMOVAL_AREA]);
    let z = e + off::PLACEMENT_POS + 8;
    let sunk = v.read_f32(z) - REMOVAL_SINK;
    v.write(z, &sunk.to_le_bytes());
    if m != 0 {
        let mz = m + matrix::POS + 8;
        let sunk = v.read_f32(mz) - REMOVAL_SINK;
        v.write(mz, &sunk.to_le_bytes());
    }
    true
}

/// Whether removals were made this session (the world's loads then need [`apply_removals`]).
pub fn removals_active() -> bool {
    REMOVALS.with_borrow(|l| !l.is_empty())
}

/// Applies the session's removals to the entity at `entity` that the world just loaded (an instance of a streamed
/// IPL); a no-op for an entity no removal matches.
pub fn apply_removals(entity: u32) -> bool {
    if entity == 0 {
        return false;
    }
    let removed = REMOVALS.with_borrow(|l| !l.is_empty() && sink_if(&mut EngineView, entity, l));
    if removed {
        REMOVED_ON_LOAD.set(REMOVED_ON_LOAD.get() + 1);
    }
    removed
}

/// The removals made so far and the entities [`apply_removals`] removed as the world loaded them.
pub fn removal_counts() -> (usize, u32) {
    (REMOVALS.with_borrow(Vec::len), REMOVED_ON_LOAD.get())
}

/// What [`materials_before_render`] changed, for [`materials_after_render`].
#[derive(Debug, Default)]
pub struct SavedMaterials {
    /// (material, texture, colour, surface properties).
    materials: Vec<(u32, u32, u32, [u32; 3])>,
    /// (geometry, flags).
    geometries: Vec<(u32, u32)>,
}

/// `rpGEOMETRYPRELIT` and `rpGEOMETRYMODULATEMATERIALCOLOR`.
const GEOMETRY_PRELIT: u32 = 0x08;
const GEOMETRY_MODULATE: u32 = 0x40;

/// RenderWare draws of an object with its own materials before the slots apply: its geometry is instanced with
/// its own flags first, as a colour slot clears the prelit flag the instancing reads (with the slots from the first
/// draw the D3D9 pipeline would instance the geometry again on every draw).
const OWN_DRAWS_FIRST: u8 = 2;

/// Before the object at `entity` is drawn: its material slots onto the materials of its RenderWare object in
/// geometry order (a texture replaces the material's; a colour modulates it with prelighting off and surface
/// properties 1, 0, 1), from its RenderWare object's third draw on ([`OWN_DRAWS_FIRST`]). `None` when nothing
/// applies.
pub fn materials_before_render(entity: u32) -> Option<SavedMaterials> {
    let v = EngineView;
    let h = addr::OBJECT_POOL.ref_of(&v, entity)?;
    let slots = MATERIALS.with_borrow(|m| m.get(&h).copied())?;
    let rw = v.read_u32(entity + off::RW_OBJECT);
    if rw == 0 {
        return None;
    }
    let instanced = OWN_DRAWS.with_borrow_mut(|d| {
        let e = d.entry(h).or_insert((rw, 0));
        if e.0 != rw {
            *e = (rw, 0);
        }
        if e.1 < OWN_DRAWS_FIRST {
            e.1 += 1;
            return false;
        }
        true
    });
    if !instanced {
        return None;
    }
    let mut saved = SavedMaterials::default();
    let mut v = EngineView;
    match v.read_u8(rw) {
        // rpATOMIC
        1 => apply_atomic(&mut v, rw, &slots, &mut saved),
        // rpCLUMP: its atomic list (+0x8), each atomic linked at +0x40
        2 => {
            let head = rw + 8;
            let mut link = v.read_u32(head);
            let mut guard = 0;
            while link != head && link != 0 && guard < 256 {
                apply_atomic(&mut v, link - 0x40, &slots, &mut saved);
                link = v.read_u32(link);
                guard += 1;
            }
        }
        _ => {}
    }
    Some(saved)
}

fn apply_atomic(v: &mut EngineView, atomic: u32, slots: &[MaterialSlot; MATERIAL_SLOTS], saved: &mut SavedMaterials) {
    let geom = v.read_u32(atomic + 0x18);
    if geom == 0 {
        return;
    }
    let mats = v.read_u32(geom + 0x20);
    let n = v.read_i32(geom + 0x24).clamp(0, MATERIAL_SLOTS as i32) as usize;
    let mut geom_saved = false;
    for (i, slot) in slots.iter().enumerate().take(n) {
        if *slot == MaterialSlot::default() {
            continue;
        }
        let m = v.read_u32(mats + 4 * i as u32);
        if m == 0 {
            continue;
        }
        saved.materials.push((m, v.read_u32(m), v.read_u32(m + 4), [0xC, 0x10, 0x14].map(|o| v.read_u32(m + o))));
        if slot.texture != 0 {
            v.write_u32(m, slot.texture);
        }
        if slot.colour != 0 {
            if !geom_saved {
                let flags = v.read_u32(geom + 8);
                saved.geometries.push((geom, flags));
                v.write_u32(geom + 8, (flags | GEOMETRY_MODULATE) & !GEOMETRY_PRELIT);
                geom_saved = true;
            }
            for (o, s) in [(0xC, 1.0f32), (0x10, 0.0), (0x14, 1.0)] {
                v.write(m + o, &s.to_le_bytes());
            }
            // RwRGBA: r, g, b, a in memory order
            v.write(m + 4, &slot.colour.to_be_bytes());
        }
    }
}

/// After the object was drawn: what [`materials_before_render`] changed, restored.
pub fn materials_after_render(saved: SavedMaterials) {
    let mut v = EngineView;
    for &(m, tex, col, sp) in saved.materials.iter().rev() {
        v.write_u32(m, tex);
        v.write_u32(m + 4, col);
        for (o, w) in [0xC, 0x10, 0x14].into_iter().zip(sp) {
            v.write_u32(m + o, w);
        }
    }
    for &(g, flags) in &saved.geometries {
        v.write_u32(g + 8, flags);
    }
}

/// Whether any object is drawn with other materials (the render hook is needed).
pub fn materials_active() -> bool {
    MATERIALS.with_borrow(|m| !m.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{self, put};

    #[test]
    fn removal_rule() {
        let r = Removal { model: -1, centre: Vec3::default(), radius: 5.0 };
        assert!(r.matches(1234, Vec3::new(3.0, 4.0, 0.0)));
        assert!(!Removal { model: 10, ..r }.matches(1234, Vec3::new(3.0, 4.0, 0.0)));
        assert!(!Removal { radius: 4.9, ..r }.matches(1234, Vec3::new(3.0, 4.0, 0.0)));
    }

    #[test]
    fn removal_sinks_matching_objects_and_keeps_the_occluder_link() {
        let mut g = fake::game();
        let obj = fake::OBJS + 3 * addr::OBJECT_POOL.element;
        put(&mut g, fake::OBJ_MAP + 3, 0x01u8);
        put(&mut g, obj + off::MODEL, 1337u16);
        put(&mut g, obj + off::PLACEMENT_POS, [10.0f32, 0.0, 5.0]);
        let other = fake::OBJS + 4 * addr::OBJECT_POOL.element;
        put(&mut g, fake::OBJ_MAP + 4, 0x01u8);
        put(&mut g, other + off::MODEL, 1338u16);
        put(&mut g, other + off::PLACEMENT_POS, [10.0f32, 0.0, 5.0]);
        put(&mut g, OCCLUDER_COUNT, 1i32);
        put(&mut g, OCCLUDERS, [40i16, 0, 20, 8, 8, 8]);
        put(&mut g, OCCLUDERS + 0x10, 7u16);
        assert_eq!(g.remove_buildings(1337, Vec3::new(10.0, 0.0, 5.0), 1.0), Ok(1));
        assert_eq!(g.view.read_u8(obj + off::AREA), REMOVAL_AREA);
        assert_eq!(g.view.read_f32(obj + off::PLACEMENT_POS + 8), 5.0 - REMOVAL_SINK);
        assert_eq!(g.view.read_f32(other + off::PLACEMENT_POS + 8), 5.0);
        assert!((0..0xC).all(|b| g.view.read_u8(OCCLUDERS + b) == 0));
        assert_eq!(g.view.read_u16(OCCLUDERS + 0x10), 7);
        assert!(removals_active());
    }

    #[test]
    fn object_snapshot_reads_the_flags() {
        let mut g = fake::game();
        let obj = fake::OBJS + 2 * addr::OBJECT_POOL.element;
        put(&mut g, fake::OBJ_MAP + 2, 0x03u8);
        put(&mut g, obj + off::MODEL, 1337u16);
        put(&mut g, obj + ENTITY_FLAGS, FLAG_VISIBLE | FLAG_USES_COLLISION);
        put(&mut g, obj + OBJECT_SCALE, 2.0f32);
        let s = g.object_snapshot(ObjectRef(0x203)).unwrap();
        assert_eq!((s.model, s.visible, s.collision, s.attached, s.scale), (ModelId(1337), true, true, false, 2.0));
        assert_eq!(g.object_snapshot(ObjectRef(0x204)), Err(GameError::InvalidHandle));
        assert_eq!(g.model_kind(ModelId(1337)), ModelKind::None);
    }
}
