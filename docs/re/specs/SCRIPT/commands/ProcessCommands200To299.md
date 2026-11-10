# CRunningScript::ProcessCommands200To299 (SCRIPT) - script switch spec

Common notation for all S1 switch files: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n, i32/f32 union).
CRunningScript fields (gr RunningScript.h): +0x14 m_IP, +0x3C m_LocalVars, +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
+0xD0 m_AndOrState, +0xD2 m_NotFlag, +0xD3 m_IsDeathArrestCheckEnabled, +0xD4 m_DoneDeathArrest, +0xDC m_ThisMustBeTheOnlyMissionRunning.
Pool helpers: 0x4048E0 = CPool<CVehicle>::GetAtRef (ECX = CPools::ms_pVehiclePool [0xB74494], stride 0xA18, returns null when
the slot flag byte != low byte of the handle), 0x404910 = CPool<CPed>::GetAtRef (0xB74490, 0x7C4), 0x465040 = CPool<CObject>::GetAtRef (0xB7449C, 0x19C).

## 0x469390 CRunningScript::ProcessCommands200To299  (catalogue: specified/REA, port level 33)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x837DD9,
  0x469390..0x4693A2), `sub esp,0x30` (0x4693A9), push ebx/ebp/esi/edi; this kept in ESI (0x4693AF).
  Index = command - 0xD6 (0x4693B1, `lea ecx,[eax-0xD6]`), `cmp ecx,0x53; ja 0x469DBD` (0x4693B7/0x4693BB): only
  0xD6..0x129 are dispatched; 200..213 (0xC8..0xD5) underflow to a large unsigned value and reach the default.
  Two-level table: `movzx ecx, byte [ecx+0x469E4C]` (0x4693C1) then `jmp [ecx*4+0x469DD4]` (0x4693C8).
- Default 0x469DBD: `or al,0xFF` (returns -1), common epilogue 0x469DBF: `mov ecx,[esp+0x40]`, pop edi/esi/ebp/ebx,
  `jmp 0x405911` (.text detour stub: `mov fs:[0],ecx; jmp 0x469DCE`), `add esp,0x3C; ret 4`. Every case below jumps
  to 0x469DBF with AL already set. Return meaning: 0 = continue with next command, -1 = unknown command.
- Opcodes: 45 with a case: 39 gta-reversed `handler`, 1 `nop` (010C GIVE_REMOTE_CONTROLLED_CAR_TO_PLAYER, case 0x469DB9 =
  `xor al,al; jmp epilogue`, i.e. a real no-op in the exe too, but note it collects no parameters), 5 `unregistered`
  specified below: 00D8, 00E2, 0111, 0112, 0119.
- Default-reaching (55, all gr `unimplemented`): 00C8..00D5 (VAR_FLOAT, LVAR_INT, LVAR_FLOAT, START_SCOPE, END_SCOPE, REPEAT,
  ENDREPEAT, IF, IFNOT, ELSE, ENDIF, WHILE, WHILENOT, ENDWHILE: compiler-only keywords), 00DA, 00DC, 00DE, 00E0, 00E3..00EB,
  00F5..00FD, 0113, 0115, 0116, 011A..0121, 0123..0128, 012A, 012B. The original returns -1 for these (the caller
  CRunningScript::ProcessOneCommand treats it as "unknown"); no gta-reversed handler falls to default in this range.

### 0x469B6D 0119 COMMAND_IS_CAR_DEAD  (gr: unregistered, used: 17/3588/111)
- Parameters: CollectParameters(1) (0x469B71); SP0 = vehicle handle.
- Behaviour: `veh = CPool<CVehicle>::GetAtRef(SP0)` (0x469B82). If `veh == null` (0x469B89) or
  `(veh->m_nStatus byte [veh+0x36] & 0xF8) == 0x28` (status 5 = STATUS_WRECKED, 0x469B8B..0x469B94) ->
  UpdateCompareFlag(true) (0x469BB9..0x469BC5). Else UpdateCompareFlag(`([veh+0x42B] >> 6) & 1` = bIsDrowning)
  (0x469B96..0x469BAD). Returns 0 (0x469BB2 / 0x469BCA).
- Note: the bool is passed as a full dword whose upper bytes come from a stale stack slot [esp+0x50] (0x469BA2/0x469BA6);
  UpdateCompareFlag only uses the low byte.
- gta-reversed: absent (unregistered). Open questions: none.

### 0x46945E 00D8 COMMAND_MISSION_HAS_FINISHED  (gr: unregistered, used: 0/132/0)
- Parameters: none.
- Behaviour: if `this->m_UsesMissionCleanup` (byte [esi+0xC6], 0x46945E) is nonzero, call `CMissionCleanup::Process`
  0x468560 with ECX = &CTheScripts::MissionCleanUp 0xA90850 (0x46946C/0x469471). Returns 0 (0x469476; the zero path
  jumps to 0x469B0C `xor al,al`).
- gta-reversed: absent. Open questions: none.

### 0x469715 00E2 COMMAND_GET_PAD_STATE  (gr: unregistered, used: 0/0/0)
- Parameters: CollectParameters(2) (0x469719); SP0 = pad index, SP1 = button id (both read as u16, 0x469720/0x469728).
- Behaviour: `SP0 = (int32)(int16) this->GetPadState((uint16)SP0, (uint16)SP1)` (CRunningScript::GetPadState 0x485B10,
  thiscall, args pushed SP1 then SP0, 0x46972F/0x469730/0x469733; result sign-extended 0x469738), StoreParameters(1)
  (0x469745). Returns 0.
- gta-reversed: absent as a command (GetPadState itself is reversed). Open questions: none.

### 0x469A7B 0111 COMMAND_SET_DEATHARREST_STATE  (gr: unregistered, used: 11/2/11)
- Parameters: CollectParameters(1) (0x469A7F); SP0 = state int.
- Behaviour: `this->m_IsDeathArrestCheckEnabled = (SP0 == 1)` (byte [esi+0xD3], 0x469A84..0x469A8E; any value other
  than exactly 1 disables). Returns 0.
- gta-reversed: absent. Open questions: none.

### 0x469A9B 0112 COMMAND_HAS_DEATHARREST_BEEN_EXECUTED  (gr: unregistered, used: 0/128/1)
- Parameters: none.
- Behaviour: UpdateCompareFlag(`this->m_DoneDeathArrest != 0`) (byte [esi+0xD4], 0x469A9B..0x469AA3, shared tail
  0x469AFC..0x469B07), returns 0 (0x469B0C).
- gta-reversed: absent. Open questions: none.
