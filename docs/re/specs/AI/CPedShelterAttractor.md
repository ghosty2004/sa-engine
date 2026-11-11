# CPedShelterAttractor (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5EE7C0 CPedShelterAttractor::CPedShelterAttractor  (catalogue: specified/REA, port level 4)
- `__thiscall (this, C2dEffectPedAttractor* fx [esp+4], CEntity* entity [esp+8], int32 moveState [esp+0xC])`, `ret 0xC`: calls `CPedAttractor::Constructor` 0x5EDFB0 (0x5ee804) with (fx=fx [esp+4], entity=entity [esp+8], moveState=moveState [esp+0xC], maxNoOfPeds=5, spacing=1.0f [0x86C470], achieveQueueTime=30000.0f [0x86C474], achieveQueueShuffleTime=3000.0f [0x86C478], arriveRange=0.5f [0x86C47C], headingRange=6.280000209f [0x86C480], deltaPos=0.100000001f [0x86C484], deltaHeading=0.100000001f [0x86C488]) - float args are pushed as raw f32 dwords loaded from .rdata; then `this->vtbl = 0x86C5B4`; returns this.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x5E9680 CPedShelterAttractor::IsAtHeadOfQueue  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x5E9680; upper EAX bits undefined), `ret 4` (4 bytes of stack args, ignored). Vtable: slot 2 (+0x8) of vtable 0x86C5B4 (CPedShelterAttractor).

## 0x5EE820 CPedShelterAttractor::GetType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4` (0x5EE820); `ret` -> returns 4. Vtable: slot 0 (+0x0) of vtable 0x86C5B4 (CPedShelterAttractor).
