//! PORT-VEH-4 (batch `veh4-2`): `CAutomobile::SpawnFlyingComponent`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, jne_mask, rd, wr, x, xi};

orig! {
    /// `RwFrameForAllObjects(RwFrame*, RwObjectCallBack, void*)` (0x7F1200)
    fn frame_for_all_objects(f: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1200 => unported;
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CObject::CObject()` (0x5A1D10)
    fn object_ctor(o: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A1D10 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// virtual `CEntity::SetModelIndexNoCreate(uint32)` (vtable +0x18)
    fn v_set_model_index_no_create(e: GPtr<u8>, mi: u32) = thiscall vslot 0x18 => unported;
    /// virtual `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
    /// `CObject::SetObjectCentreOfMass`-style offset setter `FUN_005a2cb0(float, float, float)` (0x5A2CB0)
    fn object_set_centre_offset(o: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x5A2CB0 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, the result through `out`)
    fn vec_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, the result through `out`)
    fn scalar_mul_vec(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0): ST0
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CObject::RefModelInfo(int32)` (0x59F330)
    fn object_ref_model_info(o: GPtr<u8>, mi: i32) = thiscall 0x59F330 => unported;
    /// `RwFrameCreate()` (0x7F0410)
    fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    /// `RpAtomicClone(RpAtomic*)` (0x749E60)
    fn rp_atomic_clone(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `CVisibilityPlugins::SetAtomicRenderCallback(RpAtomic*, RpAtomicCallBackRender)` (0x7328A0)
    fn set_atomic_render_callback(a: GPtr<u8>, cb: u32) = cdecl 0x7328A0 => unported;
    /// `CEntity::AttachToRwObject(RwObject*, bool)` (0x533ED0)
    fn entity_attach_to_rw_object(e: GPtr<u8>, rw: GPtr<u8>, update: u32) = thiscall 0x533ED0 => unported;
    /// `CObject::SetRemapTexture(RwTexture*, int16 txd)` (0x59F350)
    fn object_set_remap_texture(o: GPtr<u8>, tex: u32, txd: i32) = thiscall 0x59F350 => unported;
    /// `CMatrix::operator+=(const CVector&)` (0x46AE00): adds to the position
    fn matrix_add_pos(m: GPtr<u8>, v: GPtr<u8>) = thiscall 0x46AE00 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => unported;
    /// `CEntity::IsVisible()` (0x536BC0, AL)
    fn entity_is_visible(e: GPtr<u8>) -> u8 = thiscall 0x536BC0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CCollision::ProcessColModels(const CMatrix&, CColModel&, const CMatrix&, CColModel&, CColPoint*,
    /// CColPoint*, float*, bool)` (0x4185C0)
    fn process_col_models(m1: GPtr<u8>, c1: GPtr<u8>, m2: GPtr<u8>, c2: GPtr<u8>, cps: u32, lcps: u32, dists: u32, b: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The position of an entity: its matrix's (+0x30) or its placement's (+4).
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CAutomobile::SpawnFlyingComponent(int32 nodeIndex, uint32 type)` (0x6A8580).
///
/// Null at 150 temporary objects, without the node's frame (+0x648) or its atomic, or when the object
/// pool is full (deal-with-no-memory set around the allocation). The `CObject` gets the model by node
/// 0x12 (0x179) or by type (0 bonnet 0x177 with a centre offset `-dot(ltmPos - carPos, carRight)` along
/// x, 1 boot 0x17B, 2 door 0x176 (offset -0.5 y, draw-last flag 0x4000), 3 wheel 0x179 (0.4 y), 4 panel
/// 0x17A (-0.3 y), else 0x178), the car's model info referenced, a clone of the atomic on a new frame
/// at the node's LTM, physics (mass 10, turn mass 25, air resistance 0.97, elasticity 0.1, buoyancy
/// 0.1067, temporary type 3, not static, flags), the car's colours and remap texture and a lifetime
/// (now + 4000 ms above 20 temporary objects, + 10000 above 10, else + 20000). It inherits the car's
/// speed (z * 1.5 when rising; when falling and the car is upright, wheels/node 0x12 get
/// `0.04 - z * 1.5`, others z * 0.25; x/y * 0.75) and twice its turn speed, is pushed away from the car
/// (for wheels/node 0x12 along the car's up axis too, moving the object by `up * |speed xy|` when the
/// car is upright); type 1 gets turn mass 5, turn speed x 0.5 and air resistance 0.99. A wrecked
/// (status 5) visible car facing away from the camera (`dir.(cam - car) > -0.5`) pushes it toward the
/// camera (`(cam - car normalised, z + 0.3) * 5`). If it touches the car it ignores it (+0x128); it is
/// added to the world.
#[hook(addr = 0x6A8580, conv = thiscall, name = "CAutomobile::SpawnFlyingComponent", this = "CAutomobile",
       expect = "6A FF 90 E9 BC DA D5 FF",
       unverified = "veh4-2", route = "L2", hazards = "heap-writes",
       recs = "driving",
       note = "entry is `push -1; nop; jmp 0x406044` (the SEH frame setup in a protection fragment, not reproduced); creates a CObject, an RW frame and an atomic clone, adds it to the world")]
pub fn automobile_spawn_flying_component(this: GPtr<u8>, node: i32, kind: u32) -> GPtr<u8> {
    let temp = g::<u16>(0xBB_4A70);
    if temp.read() >= 0x96 {
        return GPtr::NULL;
    }
    let frame = rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize);
    if frame.is_null() {
        return GPtr::NULL;
    }
    let mut atomic = [0u32; 1];
    frame_for_all_objects(frame, 0x6A_0750, buf(&mut atomic));
    if atomic[0] == 0 {
        return GPtr::NULL;
    }
    let pool = || g::<GPtr<u8>>(0xB7_449C).read();
    wr::<u8>(pool(), 0x11, 1);
    let mem = object_new(0x17C);
    let obj = if mem.is_null() { mem } else { object_ctor(mem) };
    wr::<u8>(pool(), 0x11, 0);
    if obj.is_null() {
        return GPtr::NULL;
    }
    let ltm = frame_get_ltm(rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize));
    if node == 0x12 {
        v_set_model_index_no_create(obj, 0x179);
    } else {
        match kind {
            0 => {
                v_set_model_index_no_create(obj, 0x177);
                let mut tmp = [0u32; 0x12];
                matrix_ctor_rw(buf(&mut tmp), ltm, 0);
                let vm = rd::<GPtr<u8>>(this, 0x14);
                let vp = if vm.is_null() { this.byte_add(4) } else { vm.byte_add(0x30) };
                let mut d = [0u32; 3];
                let r = vec_sub(buf(&mut d), buf(&mut tmp).byte_add(0x30), vp);
                let off = f32_of(X87::from_f64_exact(dot_product(r, vm)).fchs());
                object_set_centre_offset(obj, off, 0.0, 0.0);
                matrix_dtor(buf(&mut tmp));
            }
            1 => v_set_model_index_no_create(obj, 0x17B),
            2 => {
                v_set_model_index_no_create(obj, 0x176);
                object_set_centre_offset(obj, 0.0, -0.5, 0.0);
                wr::<u32>(obj, 0x1C, rd::<u32>(obj, 0x1C) | 0x4000);
            }
            3 => {
                v_set_model_index_no_create(obj, 0x179);
                object_set_centre_offset(obj, 0.0, f32::from_bits(0x3ECC_CCCD), 0.0);
            }
            4 => {
                v_set_model_index_no_create(obj, 0x17A);
                object_set_centre_offset(obj, 0.0, f32::from_bits(0xBE99_999A), 0.0);
            }
            _ => v_set_model_index_no_create(obj, 0x178),
        }
    }
    object_ref_model_info(obj, i32::from(rd::<i16>(this, 0x22)));
    let nf = rw_frame_create();
    let clone = rp_atomic_clone(GPtr::from_addr(atomic[0] as usize));
    for k in 0..16isize {
        wr::<u32>(nf, 0x10 + 4 * k, rd::<u32>(ltm, 4 * k));
    }
    rp_atomic_set_frame(clone, nf);
    set_atomic_render_callback(clone, 0);
    entity_attach_to_rw_object(obj, clone, 1);
    wr::<u32>(obj, 0x1C, rd::<u32>(obj, 0x1C) | 0x8_0000);
    wr::<u32>(obj, 0x8C, 0x4120_0000);
    wr::<u32>(obj, 0x90, 0x41C8_0000);
    wr::<u32>(obj, 0x98, 0x3F78_51EC);
    wr::<u32>(obj, 0x9C, 0x3DCC_CCCD);
    wr::<u32>(obj, 0xA0, 0x3DDA_740F);
    wr::<u8>(obj, 0x13C, 3);
    v_set_is_static(obj, 0);
    wr::<u32>(obj, 0x140, (rd::<u32>(obj, 0x140) & 0xFFFF_FFFE) | 0x80);
    wr::<u8>(obj, 0x14C, rd::<u8>(this, 0x434));
    wr::<u8>(obj, 0x14D, rd::<u8>(this, 0x435));
    object_set_remap_texture(obj, rd::<u32>(this, 0x59C), i32::from(rd::<i16>(this, 0x598)));
    let n = temp.read().wrapping_add(1);
    temp.write(n);
    let now = g::<u32>(0xB7_CB84).read();
    let life = |add: f32| {
        let mut t = xi(now as i32);
        if (now as i32) < 0 {
            t = t.fadd_m(4_294_967_296.0);
        }
        ftol32(t.fadd_m(add)) as u32
    };
    let dies = if n > 20 {
        life(4000.0)
    } else if n > 10 {
        life(10000.0)
    } else {
        now.wrapping_add(20000)
    };
    wr::<u32>(obj, 0x150, dies);
    for o in [0x44isize, 0x48, 0x4C] {
        wr::<u32>(obj, o, rd::<u32>(this, o));
    }
    let wheelish = kind == 3 || kind == 4 || node == 0x12;
    let vz = rd::<f32>(obj, 0x4C);
    let z = if !jne_mask(x(vz), x(0.0), 0x41) {
        x(vz).fmul_m(1.5)
    } else if !jne_mask(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28)), x(0.0), 0x41) && wheelish {
        x(vz).fmul_m(1.5).fsubr_m(f32::from_bits(0x3D23_D70A))
    } else {
        x(vz).fmul_m(0.25)
    };
    wr::<f32>(obj, 0x4C, f32_of(z));
    wr::<f32>(obj, 0x44, f32_of(x(rd::<f32>(obj, 0x44)).fmul_m(0.75)));
    wr::<f32>(obj, 0x48, f32_of(x(rd::<f32>(obj, 0x48)).fmul_m(0.75)));
    for o in [0x50isize, 0x54, 0x58] {
        let v = f32_of(x(rd::<f32>(this, o)).fdouble());
        wr::<f32>(obj, o, v);
    }
    let vp = pos_ptr(this);
    let op = pos_ptr(obj);
    let mut dir = [
        f32_of(x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(vp, 0))).to_bits(),
        f32_of(x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(vp, 4))).to_bits(),
        f32_of(x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(vp, 8))).to_bits(),
    ];
    vector_normalise(buf(&mut dir));
    if wheelish {
        let m = rd::<GPtr<u8>>(this, 0x14);
        for (k, d) in dir.iter_mut().enumerate() {
            *d = f32_of(x(f32::from_bits(*d)).fadd_m(rd::<f32>(m, 0x20 + 4 * k as isize))).to_bits();
        }
        if !jne_mask(x(rd::<f32>(m, 0x28)), x(0.0), 0x41) {
            let sx = x(rd::<f32>(this, 0x44));
            let sy = x(rd::<f32>(this, 0x48));
            let sp = f32_of(sx.fmul(sx).fadd(sy.fmul(sy)).fsqrt());
            let mut out = [0u32; 3];
            let r = scalar_mul_vec(buf(&mut out), sp, m.byte_add(0x20));
            matrix_add_pos(rd::<GPtr<u8>>(obj, 0x14), r);
        }
    }
    let f = |v: &[u32; 3]| CVector { x: f32::from_bits(v[0]), y: f32::from_bits(v[1]), z: f32::from_bits(v[2]) };
    apply_move_force(obj, f(&dir));
    if kind == 1 {
        wr::<u32>(obj, 0x90, 0x40A0_0000);
        wr::<u32>(obj, 0x50, 0x3F00_0000);
        wr::<u32>(obj, 0x98, 0x3F7D_70A4);
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0x28 && entity_is_visible(this) != 0 {
        let cm = g::<GPtr<u8>>(0xB6_F03C).read();
        let cp = if cm.is_null() { GPtr::<u8>::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
        let vp = pos_ptr(this);
        let mut e = [0u32; 3];
        let r = vec_sub(buf(&mut e), cp, vp);
        let dot = x(f32::from_bits(dir[1]))
            .fmul_m(rd::<f32>(r, 4))
            .fadd(x(f32::from_bits(dir[2])).fmul_m(rd::<f32>(r, 8)))
            .fadd(x(f32::from_bits(dir[0])).fmul_m(rd::<f32>(r, 0)));
        if !jne_mask(dot, x(-0.5), 0x41) {
            let mut e2 = [0u32; 3];
            let r = vec_sub(buf(&mut e2), cp, pos_ptr(this));
            dir = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
            vector_normalise(buf(&mut dir));
            dir[2] = f32_of(x(f32::from_bits(dir[2])).fadd_m(f32::from_bits(0x3E99_999A))).to_bits();
            let mut out = [0u32; 3];
            let r = scalar_mul_vec(buf(&mut out), g::<f32>(0x87_1020).read(), buf(&mut dir));
            let v = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
            apply_move_force(obj, f(&v));
        }
    }
    let omi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * i32::from(rd::<i16>(obj, 0x22)) as usize).read();
    let ocol = rd::<GPtr<u8>>(omi, 0x14);
    let om = rd::<GPtr<u8>>(obj, 0x14);
    let vm = rd::<GPtr<u8>>(this, 0x14);
    let vcol = entity_get_col_model(this);
    if process_col_models(om, ocol, vm, vcol, 0xB9_ACD0, 0, 0, 0) > 0 {
        wr::<GPtr<u8>>(obj, 0x128, this);
    }
    if rd::<u32>(this, 0x40) & 0x2000_0000 != 0 {
        wr::<u32>(obj, 0x40, rd::<u32>(obj, 0x40) | 0x2000_0000);
    }
    world_add(obj);
    obj
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_SPAWN_FLYING_COMPONENT];
