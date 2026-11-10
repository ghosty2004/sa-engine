# CTaskSimpleChat (AI) - RE-SPEC-3 group I4

## 0x5F65A0 CTaskSimpleChat::CTaskSimpleChat  (catalogue: specified/REA, port level 2)
- `__thiscall CTaskSimpleChat* (this, uint32 duration [esp+4])`, `ret 4`, returns this.
- Callers: CTaskComplexPartnerGreet::GetPartnerSequence 0x682630, CTaskComplexChat::CreateFirstSubTask 0x682E20,
  CTaskComplexPartnerChat::GetPartnerSequence 0x684380.
- Behaviour: `CTaskSimpleRunTimedAnim::CTaskSimpleRunTimedAnim(this, group 0, anim 0xC (12), blendDelta 4.0f (imm 0x40800000),
  blendOutDelta -4.0f (imm 0xC0800000), duration, taskType 0x1A0 (416), name "Chat" (0x86C7B4), holdLastFrame 0)` 0x61ABE0
  (pushed right-to-left 0x5F65A5-0x5F65BE); `vtbl = 0x86C78C` (0x5F65C7); return this.
- gta-reversed: TaskSimpleChat.cpp:14 (ANIM_GROUP_DEFAULT, ANIM_ID_IDLE_CHAT, 4.f, -4.f, duration, TASK_SIMPLE_CHAT, "Chat", 0): matches.
