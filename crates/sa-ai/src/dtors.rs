//! AI task and event destructors of the two compiler shapes: complete destructors (`vtbl = X; jmp base`)
//! and scalar deleting destructors (`call dtor; if (flags & 1) pool delete; return this`).
//! Generated from the 1.0 US bytes (every body below was matched instruction by instruction; the generator is
//! described in `docs/arch/ai-port.md`). Edit by hand only together with that table.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplexWanderCop::CTaskComplexWanderCop` (0x460D80), called with `this` only (thiscall)
    pub fn dtor_460d80(this: GPtr<u8>) = thiscall 0x460D80 => unported;
    /// `CTaskSimpleLeaveGroup::Destructor` (0x463560), called with `this` only (thiscall)
    pub fn dtor_463560(this: GPtr<u8>) = thiscall 0x463560 => unported;
    /// `FUN_0046f690` (0x46F690), called with `this` only (thiscall)
    pub fn dtor_46f690(this: GPtr<u8>) = thiscall 0x46F690 => unported;
    /// `thunk_FUN_0061a6f0` (0x48DE60), called with `this` only (thiscall)
    pub fn dtor_48de60(this: GPtr<u8>) = thiscall 0x48DE60 => unported;
    /// `CTaskSimpleSay::Destructor` (0x48E4E0), called with `this` only (thiscall)
    pub fn dtor_48e4e0(this: GPtr<u8>) = thiscall 0x48E4E0 => unported;
    /// `FUN_0048e600` (0x48E600), called with `this` only (thiscall)
    pub fn dtor_48e600(this: GPtr<u8>) = thiscall 0x48E600 => unported;
    /// `CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal` (0x48E720), called with `this` only (thiscall)
    pub fn dtor_48e720(this: GPtr<u8>) = thiscall 0x48E720 => unported;
    /// `FUN_0048e8a0` (0x48E8A0), called with `this` only (thiscall)
    pub fn dtor_48e8a0(this: GPtr<u8>) = thiscall 0x48E8A0 => unported;
    /// `thunk_FUN_0061a6f0` (0x48E930), called with `this` only (thiscall)
    pub fn dtor_48e930(this: GPtr<u8>) = thiscall 0x48E930 => unported;
    /// `thunk_FUN_0061a6f0` (0x48EA90), called with `this` only (thiscall)
    pub fn dtor_48ea90(this: GPtr<u8>) = thiscall 0x48EA90 => unported;
    /// `CTaskSimpleChat::Destructor` (0x5F7FD0), called with `this` only (thiscall)
    pub fn dtor_5f7fd0(this: GPtr<u8>) = thiscall 0x5F7FD0 => unported;
    /// `FUN_0061a3a0` (0x61A3A0), called with `this` only (thiscall)
    pub fn dtor_61a3a0(this: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `FUN_0061a3d0` (0x61A3D0), called with `this` only (thiscall)
    pub fn dtor_61a3d0(this: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `FUN_0061aeb0` (0x61AEB0), called with `this` only (thiscall)
    pub fn dtor_61aeb0(this: GPtr<u8>) = thiscall 0x61AEB0 => unported;
    /// `CTaskComplexPlayHandSignalAnim::Destructor` (0x61BDF0), called with `this` only (thiscall)
    pub fn dtor_61bdf0(this: GPtr<u8>) = thiscall 0x61BDF0 => unported;
    /// `CTaskSimpleRunNamedAnim::Destructor` (0x61BF10), called with `this` only (thiscall)
    pub fn dtor_61bf10(this: GPtr<u8>) = thiscall 0x61BF10 => unported;
    /// `CTaskComplexRoadRage::Destructor` (0x622110), called with `this` only (thiscall)
    pub fn dtor_622110(this: GPtr<u8>) = thiscall 0x622110 => unported;
    /// `FUN_0062f360` (0x62F360), called with `this` only (thiscall)
    pub fn dtor_62f360(this: GPtr<u8>) = thiscall 0x62F360 => unported;
    /// `CTaskSimpleSitIdle::Destructor` (0x6311B0), called with `this` only (thiscall)
    pub fn dtor_6311b0(this: GPtr<u8>) = thiscall 0x6311B0 => unported;
    /// `CTaskComplexSitDownThenIdleThenStandUp::Destructor` (0x6314A0), called with `this` only (thiscall)
    pub fn dtor_6314a0(this: GPtr<u8>) = thiscall 0x6314A0 => unported;
    /// `CTaskComplexObserveTrafficLights::~CTaskComplexObserveTrafficLights` (0x6317C0), called with `this` only (thiscall)
    pub fn dtor_6317c0(this: GPtr<u8>) = thiscall 0x6317C0 => unported;
    /// `CTaskComplexObserveTrafficLightsAndAchieveHeading::~CTaskComplexObserveTrafficLightsAndAchieveHeading` (0x631980), called with `this` only (thiscall)
    pub fn dtor_631980(this: GPtr<u8>) = thiscall 0x631980 => unported;
    /// `CTaskComplexUseEffect::Destructor` (0x632270), called with `this` only (thiscall)
    pub fn dtor_632270(this: GPtr<u8>) = thiscall 0x632270 => unported;
    /// `CTaskComplexUseAttractor::Destructor` (0x632720), called with `this` only (thiscall)
    pub fn dtor_632720(this: GPtr<u8>) = thiscall 0x632720 => unported;
    /// `CTaskComplexBeInGroup::Destructor` (0x632EA0), called with `this` only (thiscall)
    pub fn dtor_632ea0(this: GPtr<u8>) = thiscall 0x632EA0 => unported;
    /// `CTaskSimpleIKManager::Destructor` (0x633830), called with `this` only (thiscall)
    pub fn dtor_633830(this: GPtr<u8>) = thiscall 0x633830 => unported;
    /// `CTaskSimpleIKLookAt::Destructor-` (0x633EF0), called with `this` only (thiscall)
    pub fn dtor_633ef0(this: GPtr<u8>) = thiscall 0x633EF0 => unported;
    /// `CTaskSimpleIKPointArm::Destructor` (0x634240), called with `this` only (thiscall)
    pub fn dtor_634240(this: GPtr<u8>) = thiscall 0x634240 => unported;
    /// `CTaskComplexSequence` (0x6389F0), called with `this` only (thiscall)
    pub fn dtor_6389f0(this: GPtr<u8>) = thiscall 0x6389F0 => unported;
    /// `CTaskComplexLeaveCar::~CTaskComplexLeaveCar` (0x63B970), called with `this` only (thiscall)
    pub fn dtor_63b970(this: GPtr<u8>) = thiscall 0x63B970 => unported;
    /// `CTaskSimpleCarDrive::Destructor` (0x63C460), called with `this` only (thiscall)
    pub fn dtor_63c460(this: GPtr<u8>) = thiscall 0x63C460 => unported;
    /// `FUN_0063cb50` (0x63CB50), called with `this` only (thiscall)
    pub fn dtor_63cb50(this: GPtr<u8>) = thiscall 0x63CB50 => unported;
    /// `FUN_0063d100` (0x63D100), called with `this` only (thiscall)
    pub fn dtor_63d100(this: GPtr<u8>) = thiscall 0x63D100 => unported;
    /// `FUN_00640330` (0x640330), called with `this` only (thiscall)
    pub fn dtor_640330(this: GPtr<u8>) = thiscall 0x640330 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::Destructor` (0x646570), called with `this` only (thiscall)
    pub fn dtor_646570(this: GPtr<u8>) = thiscall 0x646570 => unported;
    /// `CTaskSimpleCarWaitToSlowDown::Destructor` (0x646A00), called with `this` only (thiscall)
    pub fn dtor_646a00(this: GPtr<u8>) = thiscall 0x646A00 => unported;
    /// `CTaskSimpleCarGetOut::Destructor` (0x647260), called with `this` only (thiscall)
    pub fn dtor_647260(this: GPtr<u8>) = thiscall 0x647260 => unported;
    /// `FUN_00647950` (0x647950), called with `this` only (thiscall)
    pub fn dtor_647950(this: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CTaskComplexGoToCarDoorAndStandStill::Destructor` (0x64A580), called with `this` only (thiscall)
    pub fn dtor_64a580(this: GPtr<u8>) = thiscall 0x64A580 => unported;
    /// `CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep` (0x6531D0), called with `this` only (thiscall)
    pub fn dtor_6531d0(this: GPtr<u8>) = thiscall 0x6531D0 => unported;
    /// `CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep` (0x653350), called with `this` only (thiscall)
    pub fn dtor_653350(this: GPtr<u8>) = thiscall 0x653350 => unported;
    /// `CTaskComplexWalkRoundBuildingAttempt::Destructor` (0x654950), called with `this` only (thiscall)
    pub fn dtor_654950(this: GPtr<u8>) = thiscall 0x654950 => unported;
    /// `FUN_006550d0` (0x6550D0), called with `this` only (thiscall)
    pub fn dtor_6550d0(this: GPtr<u8>) = thiscall 0x6550D0 => unported;
    /// `CTaskComplexWalkRoundCar::Destructor` (0x656B00), called with `this` only (thiscall)
    pub fn dtor_656b00(this: GPtr<u8>) = thiscall 0x656B00 => unported;
    /// `CTaskComplexWanderFlee::Destructor` (0x65B370), called with `this` only (thiscall)
    pub fn dtor_65b370(this: GPtr<u8>) = thiscall 0x65B370 => unported;
    /// `CTaskComplexSmartFleePoint::Destructor` (0x65BDB0), called with `this` only (thiscall)
    pub fn dtor_65bdb0(this: GPtr<u8>) = thiscall 0x65BDB0 => unported;
    /// `CTaskComplexSmartFleeEntity::Destructor` (0x65C4D0), called with `this` only (thiscall)
    pub fn dtor_65c4d0(this: GPtr<u8>) = thiscall 0x65C4D0 => unported;
    /// `CTaskComplexGangLeader::Destructor` (0x65DF30), called with `this` only (thiscall)
    pub fn dtor_65df30(this: GPtr<u8>) = thiscall 0x65DF30 => unported;
    /// `CTaskComplexGangFollower::Destructor` (0x65EBB0), called with `this` only (thiscall)
    pub fn dtor_65ebb0(this: GPtr<u8>) = thiscall 0x65EBB0 => unported;
    /// `CTaskGangHasslePed::Destructor` (0x65FF60), called with `this` only (thiscall)
    pub fn dtor_65ff60(this: GPtr<u8>) = thiscall 0x65FF60 => unported;
    /// `CTaskSimpleDoHandSignal::Destructor` (0x6608B0), called with `this` only (thiscall)
    pub fn dtor_6608b0(this: GPtr<u8>) = thiscall 0x6608B0 => unported;
    /// `CTaskComplexSignalAtPed::Destructor` (0x660AB0), called with `this` only (thiscall)
    pub fn dtor_660ab0(this: GPtr<u8>) = thiscall 0x660AB0 => unported;
    /// `CTaskGoToVehicleAndLean::Destructor` (0x660EE0), called with `this` only (thiscall)
    pub fn dtor_660ee0(this: GPtr<u8>) = thiscall 0x660EE0 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Destructor` (0x661F30), called with `this` only (thiscall)
    pub fn dtor_661f30(this: GPtr<u8>) = thiscall 0x661F30 => unported;
    /// `FUN_00667d50` (0x667D50), called with `this` only (thiscall)
    pub fn dtor_667d50(this: GPtr<u8>) = thiscall 0x667D50 => unported;
    /// `CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading` (0x667E70), called with `this` only (thiscall)
    pub fn dtor_667e70(this: GPtr<u8>) = thiscall 0x667E70 => unported;
    /// `FUN_00668240` (0x668240), called with `this` only (thiscall)
    pub fn dtor_668240(this: GPtr<u8>) = thiscall 0x668240 => unported;
    /// `thunk_FUN_0040387c` (0x669D20), called with `this` only (thiscall)
    pub fn dtor_669d20(this: GPtr<u8>) = thiscall 0x669D20 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::Destructor` (0x66A1D0), called with `this` only (thiscall)
    pub fn dtor_66a1d0(this: GPtr<u8>) = thiscall 0x66A1D0 => unported;
    /// `CTaskComplexGoToAttractor::Destructor` (0x66B6A0), called with `this` only (thiscall)
    pub fn dtor_66b6a0(this: GPtr<u8>) = thiscall 0x66B6A0 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::Destructor` (0x66B960), called with `this` only (thiscall)
    pub fn dtor_66b960(this: GPtr<u8>) = thiscall 0x66B960 => unported;
    /// `FUN_0066e050` (0x66E050), called with `this` only (thiscall)
    pub fn dtor_66e050(this: GPtr<u8>) = thiscall 0x66E050 => unported;
    /// `CTaskComplexFollowNodeRoute::Destructor` (0x66EB70), called with `this` only (thiscall)
    pub fn dtor_66eb70(this: GPtr<u8>) = thiscall 0x66EB70 => unported;
    /// `FUN_0066f670` (0x66F670), called with `this` only (thiscall)
    pub fn dtor_66f670(this: GPtr<u8>) = thiscall 0x66F670 => unported;
    /// `FUN_00678cc0` (0x678CC0), called with `this` only (thiscall)
    pub fn dtor_678cc0(this: GPtr<u8>) = thiscall 0x678CC0 => unported;
    /// `CTaskSimpleInAir::~CTaskSimpleInAir` (0x678D50), called with `this` only (thiscall)
    pub fn dtor_678d50(this: GPtr<u8>) = thiscall 0x678D50 => unported;
    /// `CTaskSimpleLand::~CTaskSimpleLand` (0x678EE0), called with `this` only (thiscall)
    pub fn dtor_678ee0(this: GPtr<u8>) = thiscall 0x678EE0 => unported;
    /// `CTaskComplexChat::Destructor` (0x682D40), called with `this` only (thiscall)
    pub fn dtor_682d40(this: GPtr<u8>) = thiscall 0x682D40 => unported;
    /// `CTaskComplexWalkAlongsidePed::Destructor` (0x6832C0), called with `this` only (thiscall)
    pub fn dtor_6832c0(this: GPtr<u8>) = thiscall 0x6832C0 => unported;
    /// `CTaskComplexBeInCouple::~CTaskComplexBeInCouple` (0x683780), called with `this` only (thiscall)
    pub fn dtor_683780(this: GPtr<u8>) = thiscall 0x683780 => unported;
    /// `FUN_00684200` (0x684200), called with `this` only (thiscall)
    pub fn dtor_684200(this: GPtr<u8>) = thiscall 0x684200 => unported;
    /// `CTaskComplexPartnerChat::~CTaskComplexPartnerChat` (0x684320), called with `this` only (thiscall)
    pub fn dtor_684320(this: GPtr<u8>) = thiscall 0x684320 => unported;
    /// `CTaskSimplePlayerOnFoot::Destructor` (0x6857D0), called with `this` only (thiscall)
    pub fn dtor_6857d0(this: GPtr<u8>) = thiscall 0x6857D0 => unported;
    /// `CTaskComplexKillCriminal::Destructor` (0x68BF30), called with `this` only (thiscall)
    pub fn dtor_68bf30(this: GPtr<u8>) = thiscall 0x68BF30 => unported;
    /// `CTaskSimpleFacial::~CTaskSimpleFacial` (0x690CB0), called with `this` only (thiscall)
    pub fn dtor_690cb0(this: GPtr<u8>) = thiscall 0x690CB0 => unported;
    /// `CTaskComplexFacial::~CTaskComplexFacial` (0x690D90), called with `this` only (thiscall)
    pub fn dtor_690d90(this: GPtr<u8>) = thiscall 0x690D90 => unported;
    /// `FUN_00691630` (0x691630), called with `this` only (thiscall)
    pub fn dtor_691630(this: GPtr<u8>) = thiscall 0x691630 => unported;
}

/// `CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal` (0x48E720): `vtbl = 0x85A23C`, then the base destructor `thunk_FUN_0040387c` (0x669D20, tail jump).
#[hook(addr = 0x48E720, conv = thiscall, name = "CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal", expect = "C7 01 3C A2 85 00")]
pub fn task_complex_wander_criminal_ctask_complex_wander_criminal_48e720(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85a23c);
    dtor_669d20(this);
}

/// `FUN_0048e8a0` (0x48E8A0): `vtbl = 0x85A278`, then the base destructor `FUN_0061a3a0` (0x61A3A0, tail jump).
#[hook(addr = 0x48E8A0, conv = thiscall, name = "FUN_0048e8a0", expect = "C7 01 78 A2 85 00")]
pub fn fun_0048e8a0_48e8a0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85a278);
    dtor_61a3a0(this);
}

/// `CTaskComplexObserveTrafficLights::~CTaskComplexObserveTrafficLights` (0x6317C0): `vtbl = 0x86DF98`, then the base destructor `FUN_0061a3d0` (0x61A3D0, tail jump).
#[hook(addr = 0x6317C0, conv = thiscall, name = "CTaskComplexObserveTrafficLights::~CTaskComplexObserveTrafficLights", expect = "C7 01 98 DF 86 00")]
pub fn task_complex_observe_traffic_lights_ctask_complex_observe_traffic_lights_6317c0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86df98);
    dtor_61a3d0(this);
}

/// `CTaskComplexObserveTrafficLightsAndAchieveHeading::~CTaskComplexObserveTrafficLightsAndAchieveHeading` (0x631980): `vtbl = 0x86DFC4`, then the base destructor `FUN_0061a3d0` (0x61A3D0, tail jump).
#[hook(addr = 0x631980, conv = thiscall, name = "CTaskComplexObserveTrafficLightsAndAchieveHeading::~CTaskComplexObserveTrafficLightsAndAchieveHeading", expect = "C7 01 C4 DF 86 00")]
pub fn task_complex_observe_traffic_lights_and_achieve_heading_ctask_complex_observe_traffic_lights_and_achieve_heading_631980(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86dfc4);
    dtor_61a3d0(this);
}

/// `FUN_0063d100` (0x63D100): `vtbl = 0x86EA14`, then the base destructor `FUN_0061a3d0` (0x61A3D0, tail jump).
#[hook(addr = 0x63D100, conv = thiscall, name = "FUN_0063d100", expect = "C7 01 14 EA 86 00")]
pub fn fun_0063d100_63d100(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86ea14);
    dtor_61a3d0(this);
}

/// `CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading` (0x667E70): `vtbl = 0x86FD74`, then the base destructor `FUN_0061a3a0` (0x61A3A0, tail jump).
#[hook(addr = 0x667E70, conv = thiscall, name = "CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading", expect = "C7 01 74 FD 86 00")]
pub fn task_simple_achieve_heading_ctask_simple_achieve_heading_667e70(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86fd74);
    dtor_61a3a0(this);
}

/// `FUN_00668240` (0x668240): `vtbl = 0x86FDA8`, then the base destructor `FUN_0061a3d0` (0x61A3D0, tail jump).
#[hook(addr = 0x668240, conv = thiscall, name = "FUN_00668240", expect = "C7 01 A8 FD 86 00")]
pub fn fun_00668240_668240(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86fda8);
    dtor_61a3d0(this);
}

/// `FUN_00678cc0` (0x678CC0): `vtbl = 0x8704AC`, then the base destructor `FUN_0061a3d0` (0x61A3D0, tail jump).
#[hook(addr = 0x678CC0, conv = thiscall, name = "FUN_00678cc0", expect = "C7 01 AC 04 87 00")]
pub fn fun_00678cc0_678cc0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x8704ac);
    dtor_61a3d0(this);
}

/// `CTaskSimpleFacial::~CTaskSimpleFacial` (0x690CB0): `vtbl = 0x870AB8`, then the base destructor `FUN_0061a3a0` (0x61A3A0, tail jump).
#[hook(addr = 0x690CB0, conv = thiscall, name = "CTaskSimpleFacial::~CTaskSimpleFacial", expect = "C7 01 B8 0A 87 00")]
pub fn task_simple_facial_ctask_simple_facial_690cb0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870ab8);
    dtor_61a3a0(this);
}

/// `CTaskComplexFacial::~CTaskComplexFacial` (0x690D90): `vtbl = 0x870ADC`, then the base destructor `FUN_0061a3d0` (0x61A3D0, tail jump).
#[hook(addr = 0x690D90, conv = thiscall, name = "CTaskComplexFacial::~CTaskComplexFacial", expect = "C7 01 DC 0A 87 00")]
pub fn task_complex_facial_ctask_complex_facial_690d90(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870adc);
    dtor_61a3d0(this);
}

/// `CTaskComplexWanderCop::~CTaskComplexWanderCop` (0x460D60): scalar deleting destructor: `CTaskComplexWanderCop::CTaskComplexWanderCop` (0x460D80), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x460D60, conv = thiscall, name = "CTaskComplexWanderCop::~CTaskComplexWanderCop", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_complex_wander_cop_ctask_complex_wander_cop_460d60(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_460d80(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleLeaveGroup::~CTaskSimpleLeaveGroup` (0x4635F0): scalar deleting destructor: `CTaskSimpleLeaveGroup::Destructor` (0x463560), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x4635F0, conv = thiscall, name = "CTaskSimpleLeaveGroup::~CTaskSimpleLeaveGroup", expect = "56 8B F1 E8 68 FF FF FF")]
pub fn task_simple_leave_group_ctask_simple_leave_group_4635f0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_463560(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleCarDriveTimed::~CTaskSimpleCarDriveTimed` (0x46F670): scalar deleting destructor: `FUN_0046f690` (0x46F690), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x46F670, conv = thiscall, name = "CTaskSimpleCarDriveTimed::~CTaskSimpleCarDriveTimed", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_car_drive_timed_ctask_simple_car_drive_timed_46f670(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_46f690(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimple::~CTaskSimple` (0x48E4C0): scalar deleting destructor: `CTaskSimpleSay::Destructor` (0x48E4E0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x48E4C0, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_ctask_simple_48e4c0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48e4e0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexWanderStandard::~CTaskComplexWanderStandard` (0x48E5E0): scalar deleting destructor: `FUN_0048e600` (0x48E600), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x48E5E0, conv = thiscall, name = "CTaskComplexWanderStandard::~CTaskComplexWanderStandard", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_complex_wander_standard_ctask_complex_wander_standard_48e5e0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48e600(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal` (0x48E700): scalar deleting destructor: `CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal` (0x48E720), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x48E700, conv = thiscall, name = "CTaskComplexWanderCriminal::~CTaskComplexWanderCriminal", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_complex_wander_criminal_ctask_complex_wander_criminal_48e700(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48e720(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimple::~CTaskSimple` (0x48E880): scalar deleting destructor: `FUN_0048e8a0` (0x48E8A0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x48E880, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_ctask_simple_48e880(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48e8a0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleScratchHead::~CTaskSimpleScratchHead` (0x48E910): scalar deleting destructor: `thunk_FUN_0061a6f0` (0x48E930), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x48E910, conv = thiscall, name = "CTaskSimpleScratchHead::~CTaskSimpleScratchHead", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_scratch_head_ctask_simple_scratch_head_48e910(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48e930(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x48EA70): scalar deleting destructor: `thunk_FUN_0061a6f0` (0x48EA90), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x48EA70, conv = thiscall, name = "CTaskSimpleAnim::~CTaskSimpleAnim", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_anim_ctask_simple_anim_48ea70(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48ea90(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x5F7FB0): scalar deleting destructor: `CTaskSimpleChat::Destructor` (0x5F7FD0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x5F7FB0, conv = thiscall, name = "CTaskSimpleAnim::~CTaskSimpleAnim", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_anim_ctask_simple_anim_5f7fb0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_5f7fd0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x61BAA0): scalar deleting destructor: `thunk_FUN_0061a6f0` (0x48DE60), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x61BAA0, conv = thiscall, name = "CTaskSimpleAnim::~CTaskSimpleAnim", expect = "56 8B F1 E8 B8 23 E7 FF")]
pub fn task_simple_anim_ctask_simple_anim_61baa0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_48de60(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimplePlayHandSignalAnim::~CTaskSimplePlayHandSignalAnim` (0x61BD80): scalar deleting destructor: `FUN_0061aeb0` (0x61AEB0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x61BD80, conv = thiscall, name = "CTaskSimplePlayHandSignalAnim::~CTaskSimplePlayHandSignalAnim", expect = "56 8B F1 E8 28 F1 FF FF")]
pub fn task_simple_play_hand_signal_anim_ctask_simple_play_hand_signal_anim_61bd80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_61aeb0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x61BEF0): scalar deleting destructor: `CTaskSimpleRunNamedAnim::Destructor` (0x61BF10), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x61BEF0, conv = thiscall, name = "CTaskSimpleAnim::~CTaskSimpleAnim", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_simple_anim_ctask_simple_anim_61bef0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_61bf10(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x61C0D0): scalar deleting destructor: `CTaskComplexPlayHandSignalAnim::Destructor` (0x61BDF0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x61C0D0, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 18 FD FF FF")]
pub fn task_complex_ctask_complex_61c0d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_61bdf0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x629060): scalar deleting destructor: `CTaskComplexRoadRage::Destructor` (0x622110), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x629060, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 A8 90 FF FF")]
pub fn task_complex_ctask_complex_629060(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_622110(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleStandStill::~CTaskSimpleStandStill` (0x637480): scalar deleting destructor: `FUN_0062f360` (0x62F360), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x637480, conv = thiscall, name = "CTaskSimpleStandStill::~CTaskSimpleStandStill", expect = "56 8B F1 E8 D8 7E FF FF")]
pub fn task_simple_stand_still_ctask_simple_stand_still_637480(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_62f360(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimple::~CTaskSimple` (0x637A40): scalar deleting destructor: `CTaskSimpleSitIdle::Destructor` (0x6311B0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x637A40, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 68 97 FF FF")]
pub fn task_simple_ctask_simple_637a40(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6311b0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x637D90): scalar deleting destructor: `CTaskComplexSitDownThenIdleThenStandUp::Destructor` (0x6314A0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x637D90, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 08 97 FF FF")]
pub fn task_complex_ctask_complex_637d90(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6314a0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexObserveTrafficLights::~CTaskComplexObserveTrafficLights` (0x637F60): scalar deleting destructor: `CTaskComplexObserveTrafficLights::~CTaskComplexObserveTrafficLights` (0x6317C0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x637F60, conv = thiscall, name = "CTaskComplexObserveTrafficLights::~CTaskComplexObserveTrafficLights", expect = "56 8B F1 E8 58 98 FF FF")]
pub fn task_complex_observe_traffic_lights_ctask_complex_observe_traffic_lights_637f60(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6317c0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexObserveTrafficLightsAndAchieveHeading::~CTaskComplexObserveTrafficLightsAndAchieveHeading` (0x637F80): scalar deleting destructor: `CTaskComplexObserveTrafficLightsAndAchieveHeading::~CTaskComplexObserveTrafficLightsAndAchieveHeading` (0x631980), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x637F80, conv = thiscall, name = "CTaskComplexObserveTrafficLightsAndAchieveHeading::~CTaskComplexObserveTrafficLightsAndAchieveHeading", expect = "56 8B F1 E8 F8 99 FF FF")]
pub fn task_complex_observe_traffic_lights_and_achieve_heading_ctask_complex_observe_traffic_lights_and_achieve_heading_637f80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_631980(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexUseEffect::~CTaskComplexUseEffect` (0x638570): scalar deleting destructor: `CTaskComplexUseEffect::Destructor` (0x632270), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x638570, conv = thiscall, name = "CTaskComplexUseEffect::~CTaskComplexUseEffect", expect = "56 8B F1 E8 F8 9C FF FF")]
pub fn task_complex_use_effect_ctask_complex_use_effect_638570(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_632270(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x6385B0): scalar deleting destructor: `CTaskComplexUseAttractor::Destructor` (0x632720), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6385B0, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 68 A1 FF FF")]
pub fn task_complex_ctask_complex_6385b0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_632720(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexBeInGroup::~CTaskComplexBeInGroup` (0x638A80): scalar deleting destructor: `CTaskComplexBeInGroup::Destructor` (0x632EA0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x638A80, conv = thiscall, name = "CTaskComplexBeInGroup::~CTaskComplexBeInGroup", expect = "56 8B F1 E8 18 A4 FF FF")]
pub fn task_complex_be_in_group_ctask_complex_be_in_group_638a80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_632ea0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleIKManager::~CTaskSimpleIKManager` (0x639330): scalar deleting destructor: `CTaskSimpleIKManager::Destructor` (0x633830), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x639330, conv = thiscall, name = "CTaskSimpleIKManager::~CTaskSimpleIKManager", expect = "56 8B F1 E8 F8 A4 FF FF")]
pub fn task_simple_ikmanager_ctask_simple_ikmanager_639330(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_633830(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleIKLookAt::~CTaskSimpleIKLookAt` (0x639490): scalar deleting destructor: `CTaskSimpleIKLookAt::Destructor-` (0x633EF0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x639490, conv = thiscall, name = "CTaskSimpleIKLookAt::~CTaskSimpleIKLookAt", expect = "56 8B F1 E8 58 AA FF FF")]
pub fn task_simple_iklook_at_ctask_simple_iklook_at_639490(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_633ef0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleIKPointArm::~CTaskSimpleIKPointArm` (0x6394B0): scalar deleting destructor: `CTaskSimpleIKPointArm::Destructor` (0x634240), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6394B0, conv = thiscall, name = "CTaskSimpleIKPointArm::~CTaskSimpleIKPointArm", expect = "56 8B F1 E8 88 AD FF FF")]
pub fn task_simple_ikpoint_arm_ctask_simple_ikpoint_arm_6394b0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_634240(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexSequence::~CTaskComplexSequence` (0x639E20): scalar deleting destructor: `CTaskComplexSequence` (0x6389F0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x639E20, conv = thiscall, name = "CTaskComplexSequence::~CTaskComplexSequence", expect = "56 8B F1 E8 C8 EB FF FF")]
pub fn task_complex_sequence_ctask_complex_sequence_639e20(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6389f0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexLeaveCar::~CTaskComplexLeaveCar` (0x6410E0): scalar deleting destructor: `CTaskComplexLeaveCar::~CTaskComplexLeaveCar` (0x63B970), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6410E0, conv = thiscall, name = "CTaskComplexLeaveCar::~CTaskComplexLeaveCar", expect = "56 8B F1 E8 88 A8 FF FF")]
pub fn task_complex_leave_car_ctask_complex_leave_car_6410e0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_63b970(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleCarDrive::~CTaskSimpleCarDrive` (0x6426E0): scalar deleting destructor: `CTaskSimpleCarDrive::Destructor` (0x63C460), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6426E0, conv = thiscall, name = "CTaskSimpleCarDrive::~CTaskSimpleCarDrive", expect = "56 8B F1 E8 78 9D FF FF")]
pub fn task_simple_car_drive_ctask_simple_car_drive_6426e0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_63c460(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexCarDriveWander::~CTaskComplexCarDriveWander` (0x643220): scalar deleting destructor: `FUN_0063cb50` (0x63CB50), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x643220, conv = thiscall, name = "CTaskComplexCarDriveWander::~CTaskComplexCarDriveWander", expect = "56 8B F1 E8 28 99 FF FF")]
pub fn task_complex_car_drive_wander_ctask_complex_car_drive_wander_643220(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_63cb50(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x6434F0): scalar deleting destructor: `FUN_0063d100` (0x63D100), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6434F0, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 08 9C FF FF")]
pub fn task_complex_ctask_complex_6434f0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_63d100(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver` (0x643C60): scalar deleting destructor: `FUN_00640330` (0x640330), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x643C60, conv = thiscall, name = "CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver", expect = "56 8B F1 E8 C8 C6 FF FF")]
pub fn task_complex_enter_car_as_driver_ctask_complex_enter_car_as_driver_643c60(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_640330(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimple::~CTaskSimple` (0x64B000): scalar deleting destructor: `CTaskSimpleCarCloseDoorFromOutside::Destructor` (0x646570), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x64B000, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 68 B5 FF FF")]
pub fn task_simple_ctask_simple_64b000(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_646570(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleCarWaitToSlowDown::~CTaskSimpleCarWaitToSlowDown` (0x64B440): scalar deleting destructor: `CTaskSimpleCarWaitToSlowDown::Destructor` (0x646A00), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x64B440, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::~CTaskSimpleCarWaitToSlowDown", expect = "56 8B F1 E8 B8 B5 FF FF")]
pub fn task_simple_car_wait_to_slow_down_ctask_simple_car_wait_to_slow_down_64b440(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_646a00(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimple::~CTaskSimple` (0x64BDC0): scalar deleting destructor: `CTaskSimpleCarGetOut::Destructor` (0x647260), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x64BDC0, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 98 B4 FF FF")]
pub fn task_simple_ctask_simple_64bdc0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_647260(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut` (0x64BF70): scalar deleting destructor: `FUN_00647950` (0x647950), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x64BF70, conv = thiscall, name = "CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut", expect = "56 8B F1 E8 D8 B9 FF FF")]
pub fn task_simple_car_set_ped_out_ctask_simple_car_set_ped_out_64bf70(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_647950(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexGoToCarDoorAndStandStill::~CTaskComplexGoToCarDoorAndStandStill` (0x64D290): scalar deleting destructor: `CTaskComplexGoToCarDoorAndStandStill::Destructor` (0x64A580), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x64D290, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::~CTaskComplexGoToCarDoorAndStandStill", expect = "56 8B F1 E8 E8 D2 FF FF")]
pub fn task_complex_go_to_car_door_and_stand_still_ctask_complex_go_to_car_door_and_stand_still_64d290(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_64a580(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep` (0x655E80): scalar deleting destructor: `CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep` (0x6531D0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x655E80, conv = thiscall, name = "CTaskSimpleEvasiveStep::~CTaskSimpleEvasiveStep", expect = "56 8B F1 E8 48 D3 FF FF")]
pub fn task_simple_evasive_step_ctask_simple_evasive_step_655e80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6531d0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep` (0x655EE0): scalar deleting destructor: `CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep` (0x653350), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x655EE0, conv = thiscall, name = "CTaskComplexEvasiveStep::~CTaskComplexEvasiveStep", expect = "56 8B F1 E8 68 D4 FF FF")]
pub fn task_complex_evasive_step_ctask_complex_evasive_step_655ee0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_653350(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x656E50): scalar deleting destructor: `CTaskComplexWalkRoundBuildingAttempt::Destructor` (0x654950), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x656E50, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 F8 DA FF FF")]
pub fn task_complex_ctask_complex_656e50(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_654950(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexWalkRoundObject::~CTaskComplexWalkRoundObject` (0x657200): scalar deleting destructor: `FUN_006550d0` (0x6550D0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x657200, conv = thiscall, name = "CTaskComplexWalkRoundObject::~CTaskComplexWalkRoundObject", expect = "56 8B F1 E8 C8 DE FF FF")]
pub fn task_complex_walk_round_object_ctask_complex_walk_round_object_657200(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6550d0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x657B60): scalar deleting destructor: `CTaskComplexWalkRoundCar::Destructor` (0x656B00), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x657B60, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 98 EF FF FF")]
pub fn task_complex_ctask_complex_657b60(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_656b00(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexWanderFlee::`scalar deleting destructor'` (0x65D0C0): scalar deleting destructor: `CTaskComplexWanderFlee::Destructor` (0x65B370), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x65D0C0, conv = thiscall, name = "CTaskComplexWanderFlee::`scalar deleting destructor'", expect = "56 8B F1 E8 A8 E2 FF FF")]
pub fn task_complex_wander_flee_scalar_deleting_destructor_65d0c0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_65b370(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x65D260): scalar deleting destructor: `CTaskComplexSmartFleePoint::Destructor` (0x65BDB0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x65D260, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 48 EB FF FF")]
pub fn task_complex_ctask_complex_65d260(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_65bdb0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexSmartFleeEntity::~CTaskComplexSmartFleeEntity` (0x65D280): scalar deleting destructor: `CTaskComplexSmartFleeEntity::Destructor` (0x65C4D0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x65D280, conv = thiscall, name = "CTaskComplexSmartFleeEntity::~CTaskComplexSmartFleeEntity", expect = "56 8B F1 E8 48 F2 FF FF")]
pub fn task_complex_smart_flee_entity_ctask_complex_smart_flee_entity_65d280(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_65c4d0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::~CTaskComplexSeekEntity` (0x661F10): scalar deleting destructor: `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Destructor` (0x661F30), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x661F10, conv = thiscall, name = "CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::~CTaskComplexSeekEntity", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_complex_seek_entity_centity_seek_pos_calculator_xyoffset_ctask_complex_seek_entity_661f10(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_661f30(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x662350): scalar deleting destructor: `CTaskComplexGangLeader::Destructor` (0x65DF30), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x662350, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 D8 BB FF FF")]
pub fn task_complex_ctask_complex_662350(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_65df30(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexGangFollower::~CTaskComplexGangFollower` (0x6629F0): scalar deleting destructor: `CTaskComplexGangFollower::Destructor` (0x65EBB0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6629F0, conv = thiscall, name = "CTaskComplexGangFollower::~CTaskComplexGangFollower", expect = "56 8B F1 E8 B8 C1 FF FF")]
pub fn task_complex_gang_follower_ctask_complex_gang_follower_6629f0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_65ebb0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskGangHasslePed::~CTaskGangHasslePed` (0x6642A0): scalar deleting destructor: `CTaskGangHasslePed::Destructor` (0x65FF60), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6642A0, conv = thiscall, name = "CTaskGangHasslePed::~CTaskGangHasslePed", expect = "56 8B F1 E8 B8 BC FF FF")]
pub fn task_gang_hassle_ped_ctask_gang_hassle_ped_6642a0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_65ff60(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimple::~CTaskSimple` (0x6644A0): scalar deleting destructor: `CTaskSimpleDoHandSignal::Destructor` (0x6608B0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6644A0, conv = thiscall, name = "CTaskSimple::~CTaskSimple", expect = "56 8B F1 E8 08 C4 FF FF")]
pub fn task_simple_ctask_simple_6644a0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6608b0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x6644C0): scalar deleting destructor: `CTaskComplexSignalAtPed::Destructor` (0x660AB0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6644C0, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 E8 C5 FF FF")]
pub fn task_complex_ctask_complex_6644c0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_660ab0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskGoToVehicleAndLean::~CTaskGoToVehicleAndLean` (0x6644E0): scalar deleting destructor: `CTaskGoToVehicleAndLean::Destructor` (0x660EE0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6644E0, conv = thiscall, name = "CTaskGoToVehicleAndLean::~CTaskGoToVehicleAndLean", expect = "56 8B F1 E8 F8 C9 FF FF")]
pub fn task_go_to_vehicle_and_lean_ctask_go_to_vehicle_and_lean_6644e0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_660ee0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleGoToPoint::~CTaskSimpleGoToPoint` (0x66D6F0): scalar deleting destructor: `FUN_00667d50` (0x667D50), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x66D6F0, conv = thiscall, name = "CTaskSimpleGoToPoint::~CTaskSimpleGoToPoint", expect = "56 8B F1 E8 58 A6 FF FF")]
pub fn task_simple_go_to_point_ctask_simple_go_to_point_66d6f0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_667d50(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading` (0x66DB60): scalar deleting destructor: `CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading` (0x667E70), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x66DB60, conv = thiscall, name = "CTaskSimpleAchieveHeading::~CTaskSimpleAchieveHeading", expect = "56 8B F1 E8 08 A3 FF FF")]
pub fn task_simple_achieve_heading_ctask_simple_achieve_heading_66db60(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_667e70(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexGoToPointAndStandStill::~CTaskComplexGoToPointAndStandStill` (0x66DB80): scalar deleting destructor: `FUN_00668240` (0x668240), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x66DB80, conv = thiscall, name = "CTaskComplexGoToPointAndStandStill::~CTaskComplexGoToPointAndStandStill", expect = "56 8B F1 E8 B8 A6 FF FF")]
pub fn task_complex_go_to_point_and_stand_still_ctask_complex_go_to_point_and_stand_still_66db80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_668240(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexWanderGang::~CTaskComplexWanderGang` (0x66F650): scalar deleting destructor: `FUN_0066f670` (0x66F670), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x66F650, conv = thiscall, name = "CTaskComplexWanderGang::~CTaskComplexWanderGang", expect = "56 8B F1 E8 18 00 00 00")]
pub fn task_complex_wander_gang_ctask_complex_wander_gang_66f650(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_66f670(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexAvoidOtherPedWhileWandering::~CTaskComplexAvoidOtherPedWhileWandering` (0x66F680): scalar deleting destructor: `CTaskComplexAvoidOtherPedWhileWandering::Destructor` (0x66A1D0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x66F680, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::~CTaskComplexAvoidOtherPedWhileWandering", expect = "56 8B F1 E8 48 AB FF FF")]
pub fn task_complex_avoid_other_ped_while_wandering_ctask_complex_avoid_other_ped_while_wandering_66f680(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_66a1d0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexGoToAttractor::~CTaskComplexGoToAttractor` (0x670400): scalar deleting destructor: `CTaskComplexGoToAttractor::Destructor` (0x66B6A0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x670400, conv = thiscall, name = "CTaskComplexGoToAttractor::~CTaskComplexGoToAttractor", expect = "56 8B F1 E8 98 B2 FF FF")]
pub fn task_complex_go_to_attractor_ctask_complex_go_to_attractor_670400(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_66b6a0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexTurnToFaceEntityOrCoord::~CTaskComplexTurnToFaceEntityOrCoord` (0x670830): scalar deleting destructor: `CTaskComplexTurnToFaceEntityOrCoord::Destructor` (0x66B960), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x670830, conv = thiscall, name = "CTaskComplexTurnToFaceEntityOrCoord::~CTaskComplexTurnToFaceEntityOrCoord", expect = "56 8B F1 E8 28 B1 FF FF")]
pub fn task_complex_turn_to_face_entity_or_coord_ctask_complex_turn_to_face_entity_or_coord_670830(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_66b960(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexFollowPointRoute::~CTaskComplexFollowPointRoute` (0x671610): scalar deleting destructor: `FUN_0066e050` (0x66E050), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x671610, conv = thiscall, name = "CTaskComplexFollowPointRoute::~CTaskComplexFollowPointRoute", expect = "56 8B F1 E8 38 CA FF FF")]
pub fn task_complex_follow_point_route_ctask_complex_follow_point_route_671610(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_66e050(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexFollowNodeRoute::~CTaskComplexFollowNodeRoute` (0x671730): scalar deleting destructor: `CTaskComplexFollowNodeRoute::Destructor` (0x66EB70), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x671730, conv = thiscall, name = "CTaskComplexFollowNodeRoute::~CTaskComplexFollowNodeRoute", expect = "56 8B F1 E8 38 D4 FF FF")]
pub fn task_complex_follow_node_route_ctask_complex_follow_node_route_671730(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_66eb70(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexInAirAndLand::~CTaskComplexInAirAndLand` (0x67CC10): scalar deleting destructor: `FUN_00678cc0` (0x678CC0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x67CC10, conv = thiscall, name = "CTaskComplexInAirAndLand::~CTaskComplexInAirAndLand", expect = "56 8B F1 E8 A8 C0 FF FF")]
pub fn task_complex_in_air_and_land_ctask_complex_in_air_and_land_67cc10(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_678cc0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleInAir::~CTaskSimpleInAir` (0x67D340): scalar deleting destructor: `CTaskSimpleInAir::~CTaskSimpleInAir` (0x678D50), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x67D340, conv = thiscall, name = "CTaskSimpleInAir::~CTaskSimpleInAir", expect = "56 8B F1 E8 08 BA FF FF")]
pub fn task_simple_in_air_ctask_simple_in_air_67d340(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_678d50(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleLand::~CTaskSimpleLand` (0x67D360): scalar deleting destructor: `CTaskSimpleLand::~CTaskSimpleLand` (0x678EE0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x67D360, conv = thiscall, name = "CTaskSimpleLand::~CTaskSimpleLand", expect = "56 8B F1 E8 78 BB FF FF")]
pub fn task_simple_land_ctask_simple_land_67d360(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_678ee0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexChat::`scalar deleting destructor'` (0x6846D0): scalar deleting destructor: `CTaskComplexChat::Destructor` (0x682D40), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6846D0, conv = thiscall, name = "CTaskComplexChat::`scalar deleting destructor'", expect = "56 8B F1 E8 68 E6 FF FF")]
pub fn task_complex_chat_scalar_deleting_destructor_6846d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_682d40(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x684760): scalar deleting destructor: `CTaskComplexWalkAlongsidePed::Destructor` (0x6832C0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x684760, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 58 EB FF FF")]
pub fn task_complex_ctask_complex_684760(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6832c0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexBeInCouple::~CTaskComplexBeInCouple` (0x6847A0): scalar deleting destructor: `CTaskComplexBeInCouple::~CTaskComplexBeInCouple` (0x683780), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x6847A0, conv = thiscall, name = "CTaskComplexBeInCouple::~CTaskComplexBeInCouple", expect = "56 8B F1 E8 D8 EF FF FF")]
pub fn task_complex_be_in_couple_ctask_complex_be_in_couple_6847a0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_683780(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexPartnerDeal::~CTaskComplexPartnerDeal` (0x684F80): scalar deleting destructor: `FUN_00684200` (0x684200), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x684F80, conv = thiscall, name = "CTaskComplexPartnerDeal::~CTaskComplexPartnerDeal", expect = "56 8B F1 E8 78 F2 FF FF")]
pub fn task_complex_partner_deal_ctask_complex_partner_deal_684f80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_684200(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexPartnerChat::~CTaskComplexPartnerChat` (0x684FC0): scalar deleting destructor: `CTaskComplexPartnerChat::~CTaskComplexPartnerChat` (0x684320), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x684FC0, conv = thiscall, name = "CTaskComplexPartnerChat::~CTaskComplexPartnerChat", expect = "56 8B F1 E8 58 F3 FF FF")]
pub fn task_complex_partner_chat_ctask_complex_partner_chat_684fc0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_684320(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimplePlayerOnFoot::~CTaskSimplePlayerOnFoot` (0x68B0C0): scalar deleting destructor: `CTaskSimplePlayerOnFoot::Destructor` (0x6857D0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x68B0C0, conv = thiscall, name = "CTaskSimplePlayerOnFoot::~CTaskSimplePlayerOnFoot", expect = "56 8B F1 E8 08 A7 FF FF")]
pub fn task_simple_player_on_foot_ctask_simple_player_on_foot_68b0c0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_6857d0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplex::~CTaskComplex` (0x68DAB0): scalar deleting destructor: `CTaskComplexKillCriminal::Destructor` (0x68BF30), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x68DAB0, conv = thiscall, name = "CTaskComplex::~CTaskComplex", expect = "56 8B F1 E8 78 E4 FF FF")]
pub fn task_complex_ctask_complex_68dab0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_68bf30(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleFacial::~CTaskSimpleFacial` (0x692E30): scalar deleting destructor: `CTaskSimpleFacial::~CTaskSimpleFacial` (0x690CB0), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x692E30, conv = thiscall, name = "CTaskSimpleFacial::~CTaskSimpleFacial", expect = "56 8B F1 E8 78 DE FF FF")]
pub fn task_simple_facial_ctask_simple_facial_692e30(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_690cb0(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskComplexFacial::~CTaskComplexFacial` (0x692F90): scalar deleting destructor: `CTaskComplexFacial::~CTaskComplexFacial` (0x690D90), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x692F90, conv = thiscall, name = "CTaskComplexFacial::~CTaskComplexFacial", expect = "56 8B F1 E8 F8 DD FF FF")]
pub fn task_complex_facial_ctask_complex_facial_692f90(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_690d90(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

/// `CTaskSimpleHoldEntity::~CTaskSimpleHoldEntity` (0x692FD0): scalar deleting destructor: `FUN_00691630` (0x691630), then `CTask::operator delete` (0x61A5B0) when
/// bit 0 of `flags` is set; returns `this`.
#[hook(addr = 0x692FD0, conv = thiscall, name = "CTaskSimpleHoldEntity::~CTaskSimpleHoldEntity", expect = "56 8B F1 E8 58 E6 FF FF")]
pub fn task_simple_hold_entity_ctask_simple_hold_entity_692fd0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_691630(this);
    if flags & 1 != 0 {
        task_delete(this);
    }
    this
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TASK_COMPLEX_WANDER_CRIMINAL_CTASK_COMPLEX_WANDER_CRIMINAL_48E720,
    HOOK_FUN_0048E8A0_48E8A0,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_CTASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_6317C0,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING_CTASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING_631980,
    HOOK_FUN_0063D100_63D100,
    HOOK_TASK_SIMPLE_ACHIEVE_HEADING_CTASK_SIMPLE_ACHIEVE_HEADING_667E70,
    HOOK_FUN_00668240_668240,
    HOOK_FUN_00678CC0_678CC0,
    HOOK_TASK_SIMPLE_FACIAL_CTASK_SIMPLE_FACIAL_690CB0,
    HOOK_TASK_COMPLEX_FACIAL_CTASK_COMPLEX_FACIAL_690D90,
    HOOK_TASK_COMPLEX_WANDER_COP_CTASK_COMPLEX_WANDER_COP_460D60,
    HOOK_TASK_SIMPLE_LEAVE_GROUP_CTASK_SIMPLE_LEAVE_GROUP_4635F0,
    HOOK_TASK_SIMPLE_CAR_DRIVE_TIMED_CTASK_SIMPLE_CAR_DRIVE_TIMED_46F670,
    HOOK_TASK_SIMPLE_CTASK_SIMPLE_48E4C0,
    HOOK_TASK_COMPLEX_WANDER_STANDARD_CTASK_COMPLEX_WANDER_STANDARD_48E5E0,
    HOOK_TASK_COMPLEX_WANDER_CRIMINAL_CTASK_COMPLEX_WANDER_CRIMINAL_48E700,
    HOOK_TASK_SIMPLE_CTASK_SIMPLE_48E880,
    HOOK_TASK_SIMPLE_SCRATCH_HEAD_CTASK_SIMPLE_SCRATCH_HEAD_48E910,
    HOOK_TASK_SIMPLE_ANIM_CTASK_SIMPLE_ANIM_48EA70,
    HOOK_TASK_SIMPLE_ANIM_CTASK_SIMPLE_ANIM_5F7FB0,
    HOOK_TASK_SIMPLE_ANIM_CTASK_SIMPLE_ANIM_61BAA0,
    HOOK_TASK_SIMPLE_PLAY_HAND_SIGNAL_ANIM_CTASK_SIMPLE_PLAY_HAND_SIGNAL_ANIM_61BD80,
    HOOK_TASK_SIMPLE_ANIM_CTASK_SIMPLE_ANIM_61BEF0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_61C0D0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_629060,
    HOOK_TASK_SIMPLE_STAND_STILL_CTASK_SIMPLE_STAND_STILL_637480,
    HOOK_TASK_SIMPLE_CTASK_SIMPLE_637A40,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_637D90,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_CTASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_637F60,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING_CTASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING_637F80,
    HOOK_TASK_COMPLEX_USE_EFFECT_CTASK_COMPLEX_USE_EFFECT_638570,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_6385B0,
    HOOK_TASK_COMPLEX_BE_IN_GROUP_CTASK_COMPLEX_BE_IN_GROUP_638A80,
    HOOK_TASK_SIMPLE_IKMANAGER_CTASK_SIMPLE_IKMANAGER_639330,
    HOOK_TASK_SIMPLE_IKLOOK_AT_CTASK_SIMPLE_IKLOOK_AT_639490,
    HOOK_TASK_SIMPLE_IKPOINT_ARM_CTASK_SIMPLE_IKPOINT_ARM_6394B0,
    HOOK_TASK_COMPLEX_SEQUENCE_CTASK_COMPLEX_SEQUENCE_639E20,
    HOOK_TASK_COMPLEX_LEAVE_CAR_CTASK_COMPLEX_LEAVE_CAR_6410E0,
    HOOK_TASK_SIMPLE_CAR_DRIVE_CTASK_SIMPLE_CAR_DRIVE_6426E0,
    HOOK_TASK_COMPLEX_CAR_DRIVE_WANDER_CTASK_COMPLEX_CAR_DRIVE_WANDER_643220,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_6434F0,
    HOOK_TASK_COMPLEX_ENTER_CAR_AS_DRIVER_CTASK_COMPLEX_ENTER_CAR_AS_DRIVER_643C60,
    HOOK_TASK_SIMPLE_CTASK_SIMPLE_64B000,
    HOOK_TASK_SIMPLE_CAR_WAIT_TO_SLOW_DOWN_CTASK_SIMPLE_CAR_WAIT_TO_SLOW_DOWN_64B440,
    HOOK_TASK_SIMPLE_CTASK_SIMPLE_64BDC0,
    HOOK_TASK_SIMPLE_CAR_SET_PED_OUT_CTASK_SIMPLE_CAR_SET_PED_OUT_64BF70,
    HOOK_TASK_COMPLEX_GO_TO_CAR_DOOR_AND_STAND_STILL_CTASK_COMPLEX_GO_TO_CAR_DOOR_AND_STAND_STILL_64D290,
    HOOK_TASK_SIMPLE_EVASIVE_STEP_CTASK_SIMPLE_EVASIVE_STEP_655E80,
    HOOK_TASK_COMPLEX_EVASIVE_STEP_CTASK_COMPLEX_EVASIVE_STEP_655EE0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_656E50,
    HOOK_TASK_COMPLEX_WALK_ROUND_OBJECT_CTASK_COMPLEX_WALK_ROUND_OBJECT_657200,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_657B60,
    HOOK_TASK_COMPLEX_WANDER_FLEE_SCALAR_DELETING_DESTRUCTOR_65D0C0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_65D260,
    HOOK_TASK_COMPLEX_SMART_FLEE_ENTITY_CTASK_COMPLEX_SMART_FLEE_ENTITY_65D280,
    HOOK_TASK_COMPLEX_SEEK_ENTITY_CENTITY_SEEK_POS_CALCULATOR_XYOFFSET_CTASK_COMPLEX_SEEK_ENTITY_661F10,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_662350,
    HOOK_TASK_COMPLEX_GANG_FOLLOWER_CTASK_COMPLEX_GANG_FOLLOWER_6629F0,
    HOOK_TASK_GANG_HASSLE_PED_CTASK_GANG_HASSLE_PED_6642A0,
    HOOK_TASK_SIMPLE_CTASK_SIMPLE_6644A0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_6644C0,
    HOOK_TASK_GO_TO_VEHICLE_AND_LEAN_CTASK_GO_TO_VEHICLE_AND_LEAN_6644E0,
    HOOK_TASK_SIMPLE_GO_TO_POINT_CTASK_SIMPLE_GO_TO_POINT_66D6F0,
    HOOK_TASK_SIMPLE_ACHIEVE_HEADING_CTASK_SIMPLE_ACHIEVE_HEADING_66DB60,
    HOOK_TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL_CTASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL_66DB80,
    HOOK_TASK_COMPLEX_WANDER_GANG_CTASK_COMPLEX_WANDER_GANG_66F650,
    HOOK_TASK_COMPLEX_AVOID_OTHER_PED_WHILE_WANDERING_CTASK_COMPLEX_AVOID_OTHER_PED_WHILE_WANDERING_66F680,
    HOOK_TASK_COMPLEX_GO_TO_ATTRACTOR_CTASK_COMPLEX_GO_TO_ATTRACTOR_670400,
    HOOK_TASK_COMPLEX_TURN_TO_FACE_ENTITY_OR_COORD_CTASK_COMPLEX_TURN_TO_FACE_ENTITY_OR_COORD_670830,
    HOOK_TASK_COMPLEX_FOLLOW_POINT_ROUTE_CTASK_COMPLEX_FOLLOW_POINT_ROUTE_671610,
    HOOK_TASK_COMPLEX_FOLLOW_NODE_ROUTE_CTASK_COMPLEX_FOLLOW_NODE_ROUTE_671730,
    HOOK_TASK_COMPLEX_IN_AIR_AND_LAND_CTASK_COMPLEX_IN_AIR_AND_LAND_67CC10,
    HOOK_TASK_SIMPLE_IN_AIR_CTASK_SIMPLE_IN_AIR_67D340,
    HOOK_TASK_SIMPLE_LAND_CTASK_SIMPLE_LAND_67D360,
    HOOK_TASK_COMPLEX_CHAT_SCALAR_DELETING_DESTRUCTOR_6846D0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_684760,
    HOOK_TASK_COMPLEX_BE_IN_COUPLE_CTASK_COMPLEX_BE_IN_COUPLE_6847A0,
    HOOK_TASK_COMPLEX_PARTNER_DEAL_CTASK_COMPLEX_PARTNER_DEAL_684F80,
    HOOK_TASK_COMPLEX_PARTNER_CHAT_CTASK_COMPLEX_PARTNER_CHAT_684FC0,
    HOOK_TASK_SIMPLE_PLAYER_ON_FOOT_CTASK_SIMPLE_PLAYER_ON_FOOT_68B0C0,
    HOOK_TASK_COMPLEX_CTASK_COMPLEX_68DAB0,
    HOOK_TASK_SIMPLE_FACIAL_CTASK_SIMPLE_FACIAL_692E30,
    HOOK_TASK_COMPLEX_FACIAL_CTASK_COMPLEX_FACIAL_692F90,
    HOOK_TASK_SIMPLE_HOLD_ENTITY_CTASK_SIMPLE_HOLD_ENTITY_692FD0,
];
