# AI static initialisers (group I1)

Common callees: `eh_vector_constructor_iterator(void* array, u32 elemSize, u32 count, ctor, dtor)` 0x821D30 (`__stdcall`, MSVC CRT;
calls `ctor(elem)` thiscall for each element in ascending order) and `atexit(fn)` 0x821D1E (cdecl, the init pops its arg with
`pop ecx`). All are `__cdecl void()` CRT initialisers called once from the CRT init table, never from game code. A port performs the
same construction at startup and registers the matching `__dtor_*` for shutdown. Float constant inits: one x87 operation on two f32
operands, stored as f32 (a single correctly rounded operation, so the result does not depend on the precision-control setting).

## 0x849A00 __init_g_969a28  (catalogue: specified/REA, port level 0)
- `[0x969A28] (f32) = 1.0f [0x858624] / 50.0f [0x8594A8]` (fld, fdiv, fstp) = 0.02f. Global near CPedToPlayerConversations
  (unnamed; a 1/50 rate constant).

## 0x84BA30 __init_CInformFriendsEventQueue__ms_informFriendsEvents  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xA9AFB8, 0xC, 8, 0x4AC0D0 (proposed CInformFriendsEvent::CInformFriendsEvent), 0x4B2950 (dtor))`;
  `atexit(0x8561F0)`.

## 0x84BA60 __init_CInformGroupEventQueue__ms_informGroupEvents  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xA9B018, 0x10, 8, CInformGroupEvent::Constructor 0x4AC350, CInformGroupEvent::Destructor 0x4B29A0)`;
  `atexit(0x856210)`.

## 0x84BA90 __init_g_a9b098  (catalogue: specified/REA, port level 0)
- `[0xA9B098] = 0.005f [0x85B88C] * 0.0625f [0x858620]` (fld, fmul, fstp f32).

## 0x84BAD0 __init_g_a9b0a0  (catalogue: specified/REA, port level 0)
- `[0xA9B0A0] = 100.0f [0x858628] / 3.6f [0x85B890]` (fld, fdiv, fstp f32).

## 0x850F60 __init_g_c09870  (catalogue: specified/REA, port level 0)
- No construction (trivial ctor); only `atexit(__dtor_g_c09870 0x8567C0)`.

## CPedGroupDefaultTaskAllocators allocator objects  (catalogue: specified/REA, port level 1)
Each: `ctor(ECX = &object)` (the ctor only writes the vtable pointer: `[obj] = vtbl`, returns obj), then `atexit(dtor)`.
| init | object | ctor | vtable written | atexit |
|---|---|---|---|---|
| 0x851270 | ms_FollowAnyMeansAllocator 0xC09908 | 0x5F6450 | 0x86C75C | 0x856800 |
| 0x851290 | ms_FollowLimitedAllocator 0xC0990C | 0x5F6480 | 0x86C764 | 0x856810 |
| 0x8512B0 | ms_StandStillAllocator 0xC09910 | 0x5F64B0 | 0x86C76C | 0x856820 |
| 0x8512D0 | ms_ChatAllocator 0xC09914 | 0x5F64E0 | 0x86C774 | 0x856830 |
| 0x8512F0 | ms_RandomAllocator 0xC09918 | 0x5F6510 | 0x86C77C | 0x856840 |
| 0x851310 | ms_SitInLeaderCarAllocator 0xC0991C | 0x5F6540 | 0x86C784 | 0x856850 |

## 0x851330 __init_CPedGroups__ms_groups  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xC09920, 0x2D4, 8, CPedGroup::Constructor 0x5FC150, CPedGroup::Destructor 0x5FC190)`; `atexit(0x856860)`.

## 0x851380 __init_g_c0afc8  (catalogue: specified/REA, port level 0)
- `[0xC0AFC8] = 1.0f [0x858624] / 50.0f [0x86C85C]`.

## 0x851680 __init_CPedScriptedTaskRecord__ms_scriptedTasks  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xC0B1E8, 0x14, 0x80, CPedScriptedTaskRecordData::Constructor 0x608330, 0x6082E0 (dtor))`;
  `atexit(0x8568B0)`.

## 0x8516B0 __init_g_c0bbf0  (catalogue: specified/REA, port level 0)
- `[0xC0BBF0] = 0.005f [0x86D048] * 0.0625f [0x858620]`.

## 0x8516D0 __init_g_c0bbf4  (catalogue: specified/REA, port level 0)
- `[0xC0BBF4] = 1.0f [0x858624] / 50.0f [0x86D050]`.

## 0x851F50 __init_g_c177bc  (catalogue: specified/REA, port level 0)
- `[0xC177BC] = 1.0f [0x858624] / 50.0f [0x86D618]`.

## 0x851FF0 __init_col1  (catalogue: specified/REA, port level 2)
- `CColModel::CColModel(ECX = col1 0xC17824)` 0x40FB60; `atexit(__dtor_col1 0x8568F0)`.

## 0x8520E0 __init_CTaskSequences__ms_taskSequence  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xC178F0, 0x40, 0x40, CTaskComplexSequence::Constructor 0x632BD0, 0x6389F0 (dtor))`; `atexit(0x856920)`.

## 0x852110 __init_CScriptedBrainTaskStore__ms_entries  (catalogue: specified/REA, port level 0)
- `eh_vector_constructor_iterator(0xC188F0, 0x10, 0x30, CScriptedBrainTaskEntry::Constructor 0x62EC40, 0x62EC50 (dtor))`; `atexit(0x856900)`.

## 0x852140 __init_g_c18bf0  (catalogue: specified/REA, port level 0)
- `[0xC18BF0] = 0.005f [0x86E66C] * 0.0625f [0x858620]`.

## 0x8521A0 __init_g_c18bfc  (catalogue: specified/REA, port level 0)
- `[0xC18BFC] = 1.0f [0x858624] / (50.0f [0x86E674] * 50.0f)` (fld, fmul, `fdivr` with 1.0f, fstp) = 1/2500.

## 0x852680 / 0x852690 / 0x8526A0 __init_g_c18ce4 / __init_g_c18ce8 / __init_g_c18cec  (catalogue: specified/REA, port level 0)
- Each copies the dword at 0x86F648 (60.0f) into 0xC18CE4 / 0xC18CE8 / 0xC18CEC respectively (integer `mov`, no x87).

## 0x852840 __init_CTaskSimpleGoTo__ms_fLookAtThresholdDotProduct  (catalogue: specified/REA, port level 0)
- `CTaskSimpleGoTo::ms_fLookAtThresholdDotProduct [0xC18D48] = (f32) fcos(0.39269909262657166 [0x870190] (f64, pi/8))` (fld qword, `fcos`,
  fstp dword) = 0.9238795f. Port: use the x87-exact cosine of the f64 constant rounded to f32 (sa_math::x87).

## ms_*ColModel (climb task col models)  (catalogue: specified/REA, port level 2)
Each `CColModel::CColModel(ECX = obj)` 0x40FB60 then `atexit(dtor)`:
0x852A10 ms_ClimbColModel 0xC19518 / atexit 0x856960; 0x852A30 ms_StandUpColModel 0xC19548 / 0x856970;
0x852A50 ms_VaultColModel 0xC19578 / 0x856980; 0x852A70 ms_FindEdgeColModel 0xC195A8 / 0x856990.

## 0x852F10 __init_CTaskComplexFollowLeaderInFormation__ms_offsets  (catalogue: specified/REA, port level 1)
- `0x694850(ECX = ms_offsets 0xC196E8)` (catalogue unknown; proposed `CTaskComplexFollowLeaderInFormation::Offsets::Offsets`), then
  `atexit(0x8569B0)`. The ctor sets the spacing f32 at +0x80 = 1.0f and fills the 8 standing offsets (+0x00..+0x3F) and the 8 moving
  offsets (+0x40..+0x7F) as +-spacing combinations (e.g. [0] = (-1, 1), [1] = (1, 1), [2] = (0, 1)...); the exact table is the callee's
  spec (not in this group) - port it from 0x694850 byte by byte.

## 0x853170 __init_CCover__m_ListOfProcessedBuildings  (catalogue: specified/REA, port level 0)
- Only `atexit(0x8569C0)` (the list object has a trivial/zero ctor).

## 0x853240 / 0x853260 / 0x853280 __init_g_c1a43c / __init_g_c1a45c / __init_g_c1a4dc  (catalogue: specified/REA, port level 0)
- `rep stosd` zero-fills 6 dwords at 0xC1A43C / 30 dwords at 0xC1A45C / 30 dwords at 0xC1A4DC, then `atexit(0x8569D0 / 0x8569E0 /
  0x8569F0)`. (.bss is already zero; the effect is only the atexit registration.)
