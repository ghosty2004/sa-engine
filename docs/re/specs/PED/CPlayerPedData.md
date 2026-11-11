# CPlayerPedData (PED) - RE-SPEC-FINAL group A

## 0x56F200 CPlayerPedData::sub_56F200 (proposed: CPlayerPedData::~CPlayerPedData)  (catalogue: unknown/REA, port level 9)
- Convention: `__thiscall void (CPlayerPedData* this)`. Path: 0x56F200 `nop; jmp 0x401616` (HOODLUM: `ecx += 0x21C`) -> `jmp
  CAEPoliceScannerAudioEntity::~CAEPoliceScannerAudioEntity` 0x4E6E00 - i.e. destroys the police-scanner audio member at +0x21C?? NOTE: CPlayerPedData
  is only 0xAC bytes; the +0x21C member belongs to the object the callers pass (callers: 0x56F850 deleting dtor, CPlayerPedData::DeAllocateData
  0x56F890) - the catalogue class attribution is doubtful.
- gta-reversed: n/a. Open questions: real owner class of this dtor (ECX+0x21C = a CAEPoliceScannerAudioEntity member) - see 0x56F200/0x401616.

## 0x56F850 CPlayerPedData::`scalar deleting destructor'  (catalogue: unknown/REA, port level 10)
- Convention: `__thiscall T* (this, uint8 flags [esp+4])`, `ret 4`, returns this. Body: dtor 0x56F200 (destroys the member at +0x21C);
  if `flags & 1` `operator delete(this)` 0x8214BD. No code callers (vtable slot).
- gta-reversed: compiler-generated. Open questions: owner class (see 0x56F200).
