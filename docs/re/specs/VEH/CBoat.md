# CBoat (VEH) - full specs

## 0x6F1120 CBoat::vfunc_0  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall CBoat* (CBoat* this, u8 flags [esp+4])`, `ret 4`. Virtual slot 0 of the CBoat vtable
  0x8721A0 (scalar deleting destructor; proposed name `CBoat::scalar_deleting_destructor`).
- Callers: virtual (CCarCtrl::CreatePoliceChase 0x42C2B0, CreateConvoy 0x42C740, GenerateOneRandomCar 0x430050,
  CSetPiece::Update 0x499A80). Callees: CBoat::Destructor 0x6F00F0, CVehicle::operator delete 0x6E2D90 (cdecl,
  returns the slot to `CPools::ms_pVehiclePool` 0xB74494).
- Behaviour: `CBoat::~CBoat(this)`; `if (flags & 1) CVehicle::operator delete(this)`; return `this` in EAX.
- gta-reversed: compiler-generated (no hook).
