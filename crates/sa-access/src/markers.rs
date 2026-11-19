//! Pickups, checkpoint markers, radar blips and radar areas: created and removed through the game's script commands,
//! read from `CPickups::aPickUps`, drawn on the radar through `CRadar::DrawAreaOnRadar`.

use sa_core::mem::GPtr;
use sa_types::script::ScriptArg::{Float as F, Int as I, Local as L};
use sa_types::{BlipRef, CheckpointRef, GameError, GameResult, ModelId, PedRef, PickupRef, Vec3, VehicleRef};

use crate::addr::{self, off};
use crate::script::op;
use crate::{Game, View};

/// Where a coordinate blip shows and how it looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlipStyle {
    /// On the whole radar and map, else only near the player.
    pub global: bool,
    /// A contact point.
    pub contact_point: bool,
}

/// `ePedState` values of a ped that is not alive (`PEDSTATE_DIE`, `PEDSTATE_DEAD`).
const PED_STATE_DIE: u32 = 54;
const PED_STATE_DEAD: u32 = 55;
/// The pickup types whose update is the mine / floating package code, which has no pick-up test.
const PICKUP_TYPES_WITHOUT_PICKUP: std::ops::RangeInclusive<u8> = 9..=14;
/// `CPickup::m_nFlags` bit 0: collected and waiting to come back.
const PICKUP_DISABLED: u8 = 1;
/// `CObject::m_nObjectFlags` bit the pickup update tests before picking up (the object cannot be picked up).
const OBJECT_NOT_PICKABLE: u32 = 0x200_0000;

impl<V: View> Game<V> {
    /// The address of the pickup `p` names (`CPickups::GetActualPickupIndex`: index in the low word, the slot's
    /// reference count in the high word), or `InvalidHandle`.
    fn pickup_addr(&self, p: PickupRef) -> GameResult<u32> {
        let index = p.0 & 0xFFFF;
        if index >= addr::PICKUP_COUNT {
            return Err(GameError::InvalidHandle);
        }
        let a = addr::PICKUPS + index * addr::PICKUP_SIZE;
        let live = self.view.read_u16(a + off::PICKUP_REFERENCE) == (p.0 >> 16) as u16
            && self.view.read_u8(a + off::PICKUP_TYPE) != 0;
        if live { Ok(a) } else { Err(GameError::InvalidHandle) }
    }

    /// Creates a pickup of `model` and type `kind` at `pos` (`CREATE_PICKUP` 0213); its model must be loaded.
    pub fn create_pickup(&mut self, model: ModelId, kind: i32, pos: Vec3) -> GameResult<PickupRef> {
        let h = self.run_op_out(op::CREATE_PICKUP, &[I(model.0), I(kind), F(pos.x), F(pos.y), F(pos.z), L(0)])?;
        self.pickup_addr(PickupRef(h)).map(|_| PickupRef(h)).map_err(|_| GameError::NotLoaded)
    }

    /// Removes a pickup (`REMOVE_PICKUP` 0215).
    pub fn remove_pickup(&mut self, p: PickupRef) -> GameResult<()> {
        self.pickup_addr(p)?;
        self.run_op(op::REMOVE_PICKUP, &[I(p.0 as i32)]).map(drop)
    }

    /// Whether the game collected the pickup (`HAS_PICKUP_BEEN_COLLECTED` 0214).
    pub fn pickup_collected(&mut self, p: PickupRef) -> GameResult<bool> {
        self.run_op(op::HAS_PICKUP_BEEN_COLLECTED, &[I(p.0 as i32)])
    }

    /// Whether the local player stands where `CPickup::Update` picks the pickup up: the pickup has its object and is
    /// not waiting to come back, its type has the pick-up test, the object may be picked up, and the player is on
    /// foot, alive, less than `[0x858CA0]` away in height and `sqrt([0x859BB4])` across.
    pub fn pickup_touched(&self, p: PickupRef) -> GameResult<bool> {
        let a = self.pickup_addr(p)?;
        let v = &self.view;
        let object = v.read_u32(a + off::PICKUP_OBJECT);
        let kind = v.read_u8(a + off::PICKUP_TYPE);
        if object == 0
            || v.read_u8(a + off::PICKUP_FLAGS) & PICKUP_DISABLED != 0
            || PICKUP_TYPES_WITHOUT_PICKUP.contains(&kind)
            || v.read_u32(object + off::OBJECT_FLAGS) & OBJECT_NOT_PICKABLE != 0
        {
            return Ok(false);
        }
        let Some(me) = self.local_player() else { return Ok(false) };
        let s = self.ped_snapshot(me)?;
        if s.vehicle.is_some() || matches!(s.ped_state, PED_STATE_DIE | PED_STATE_DEAD) {
            return Ok(false);
        }
        let (pp, op) = (s.matrix.pos, self.entity_matrix(object).pos);
        let (dx, dy, dz) = (f64::from(pp.x - op.x), f64::from(pp.y - op.y), f64::from(pp.z - op.z));
        let height = f64::from(v.read_f32(addr::PICKUP_HEIGHT_RANGE));
        let across = f64::from(v.read_f32(addr::PICKUP_RANGE_SQUARED));
        Ok(dz.abs() < height && dx * dx + dy * dy < across)
    }

    /// Creates a checkpoint marker (`CREATE_CHECKPOINT` 06D5): `kind` the game's checkpoint type, pointing to
    /// `next`, `size` its radius.
    pub fn create_checkpoint(&mut self, kind: i32, pos: Vec3, next: Vec3, size: f32) -> GameResult<CheckpointRef> {
        let args = [I(kind), F(pos.x), F(pos.y), F(pos.z), F(next.x), F(next.y), F(next.z), F(size), L(0)];
        self.run_op_out(op::CREATE_CHECKPOINT, &args).map(CheckpointRef)
    }

    /// Deletes a checkpoint marker (`DELETE_CHECKPOINT` 06D6).
    pub fn delete_checkpoint(&mut self, c: CheckpointRef) -> GameResult<()> {
        self.run_op(op::DELETE_CHECKPOINT, &[I(c.0 as i32)]).map(drop)
    }

    /// Whether the ped is within the box `radius` around `pos` (`LOCATE_CHAR_ANY_MEANS_3D` 00FE); `marker` also
    /// places the game's area marker there for this frame.
    pub fn locate_ped(&mut self, ped: PedRef, pos: Vec3, radius: Vec3, marker: bool) -> GameResult<bool> {
        self.ped_addr(ped)?;
        let args = [
            I(ped.0 as i32),
            F(pos.x),
            F(pos.y),
            F(pos.z),
            F(radius.x),
            F(radius.y),
            F(radius.z),
            I(i32::from(marker)),
        ];
        self.run_op(op::LOCATE_CHAR_ANY_MEANS_3D, &args)
    }

    /// Adds a radar blip at `pos` with radar sprite `sprite`.
    pub fn add_coord_blip(&mut self, pos: Vec3, sprite: i32, style: BlipStyle) -> GameResult<BlipRef> {
        let opcode = match (style.global, style.contact_point) {
            (false, false) => op::ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD,
            (true, false) => op::ADD_SPRITE_BLIP_FOR_COORD,
            (false, true) => op::ADD_SHORT_RANGE_SPRITE_BLIP_FOR_CONTACT_POINT,
            (true, true) => op::ADD_SPRITE_BLIP_FOR_CONTACT_POINT,
        };
        self.run_op_out(opcode, &[F(pos.x), F(pos.y), F(pos.z), I(sprite), L(0)]).map(BlipRef)
    }

    /// Adds a radar blip on a ped (`ADD_BLIP_FOR_CHAR` 0187).
    pub fn add_ped_blip(&mut self, ped: PedRef) -> GameResult<BlipRef> {
        self.ped_addr(ped)?;
        self.run_op_out(op::ADD_BLIP_FOR_CHAR, &[I(ped.0 as i32), L(0)]).map(BlipRef)
    }

    /// Adds a radar blip on a vehicle (`ADD_BLIP_FOR_CAR` 0186).
    pub fn add_vehicle_blip(&mut self, v: VehicleRef) -> GameResult<BlipRef> {
        self.vehicle_addr(v)?;
        self.run_op_out(op::ADD_BLIP_FOR_CAR, &[I(v.0 as i32), L(0)]).map(BlipRef)
    }

    /// Removes a radar blip (`REMOVE_BLIP` 0164).
    pub fn remove_blip(&mut self, b: BlipRef) -> GameResult<()> {
        self.run_op(op::REMOVE_BLIP, &[I(b.0 as i32)]).map(drop)
    }

    /// Sets a blip's colour (`CHANGE_BLIP_COLOUR` 0165): the game's blip colours 0..=8, else `0xRRGGBBAA`.
    pub fn set_blip_colour(&mut self, b: BlipRef, colour: u32) -> GameResult<()> {
        self.run_op(op::CHANGE_BLIP_COLOUR, &[I(b.0 as i32), I(colour as i32)]).map(drop)
    }

    /// Sets a blip's size (`CHANGE_BLIP_SCALE` 0168).
    pub fn set_blip_scale(&mut self, b: BlipRef, scale: i32) -> GameResult<()> {
        self.run_op(op::CHANGE_BLIP_SCALE, &[I(b.0 as i32), I(scale)]).map(drop)
    }

    /// Sets where a blip shows (`CHANGE_BLIP_DISPLAY` 018B).
    pub fn set_blip_display(&mut self, b: BlipRef, display: i32) -> GameResult<()> {
        self.run_op(op::CHANGE_BLIP_DISPLAY, &[I(b.0 as i32), I(display)]).map(drop)
    }
}

/// Draws the world rectangle `(min_x, min_y) - (max_x, max_y)` in `rgba` (`0xRRGGBBAA`) on the radar, or on the map
/// with `in_menu` (`CRadar::DrawAreaOnRadar`). Only valid where the game draws its radar areas (its gang overlay).
pub fn draw_radar_area(min: (f32, f32), max: (f32, f32), rgba: u32, in_menu: bool) {
    // `CRect` as the radar takes it: left, bottom, right, top, with the world's y growing up the map
    let mut rect = [min.0, max.1, max.0, min.1];
    let mut colour = rgba.to_be_bytes();
    sa_core::bindings::draw_area_on_radar(
        GPtr::from_raw(rect.as_mut_ptr()),
        GPtr::from_raw(colour.as_mut_ptr()),
        u8::from(in_menu),
    );
}
