# WEAPON: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x5A4030 CProjectile::CProjectile (proposed)  (catalogue: specified/REA, port level 15)
- Convention: `__thiscall CProjectile* (this = ECX, int32 modelIndex [esp+4])`, `ret 4`, returns this. SEH frame; handler push relocated through .HOODLUM stub 0x4037B3 (plain code, sbb of two obfuscated dwords), resume 0x5A4037. Only caller CProjectileInfo::AddProjectile 0x737C80 (3 sites).
- Behaviour (store order): `CObject::CObject(this)` (0x5A1D10); `dword +0x1C |= 0x80000000` (CEntity flags, top bit); `vtbl = 0x867030`; `m_fMass (+0x8C) = 1.0f; m_fTurnMass (+0x90) = 1.0f; m_fAirResistance (+0x98) = 0.99999f (0x3F7FFF58); m_fElasticity (+0x9C) = 0.75f; m_fBuoyancyConstant (+0xA0) = 0.0008f (0x3A51B718)`; `CObject::SetIsStatic(false)` (0x5A0760); `dword +0x40 (physical flags) |= 0x00800000`; `CEntity::SetModelIndex(modelIndex)` (0x532AE0); `byte +0x13C (m_nObjectType) = 5` (OBJECT_TEMPORARY / projectile type per gr).
- gta-reversed: gr CProjectile ctor sets the same values (gr names for the flag bits: m_bUsesCollision-style bits; verify bit names, offsets certain). Open questions: none.

## 0x5A40E0 CProjectile::~CProjectile (proposed)  (catalogue: unknown/REA, port level 15)
- Name evidence: stores 0x867030 (the vtable CProjectile's ctor 0x5A4030 installs) and tail-jumps to CObject::~CObject 0x59F660; only caller is CProjectile's scalar deleting destructor 0x5A40F0.
- Convention: `__thiscall void (this = ECX)`. Behaviour: `vtbl = 0x867030; jmp CObject::~CObject` (ECX unchanged). gta-reversed: CProjectile has an implicit dtor (equivalent). Open questions: none.

## 0x742A90 CWeaponEffects::CWeaponEffects  (catalogue: specified/REA, port level 0)
- gr WeaponEffects.h:34. Element ctor referenced by __init_gCrossHair 0x855D20; empty body (`mov eax, ecx; ret`), writes nothing. Open questions: none.

## 0x743C30 CWeaponInfo::CWeaponInfo  (catalogue: specified/REA, port level 0)
- gr WeaponInfo.h:111. Element ctor referenced by __init_CWeaponInfo__aWeaponInfo 0x855E10; empty body, writes nothing (static storage stays zero until LoadWeaponData). Open questions: none.
