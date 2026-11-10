# HUD unattributed / free functions (group H1)

## 0x4410E0 CPad::sub_4410E0 (proposed: CPad::IsDPadRightPressed)  (catalogue: unknown/REA, port level 0)
- Entry `jmp 0x15667A0` (.HOODLUM body, same behaviour). `__thiscall` (ECX = CPad*), no stack args, plain `ret`;
  returns a full 32-bit EAX (1 or 0), not just AL.
- Body: `if (pad->NewState.DPadRight (+0x16, i16) != 0 && pad->OldState.DPadRight (+0x46, i16) == 0) return 1; return 0;`
  (0x15667A0 `cmp word [ecx+0x16],0`, 0x15667A7 `cmp word [ecx+0x46],0`). CPad: NewState CControllerState at +0,
  OldState at +0x30; DPadRight is controller-state +0x16.
- Name evidence: exact BUTTON_IS_PRESSED(DPadRight) pattern (gr Pad.h:294 `IsDPadRightPressed` has no address).
  Caller: CMenuManager::UserInput 0x57FD70 (1 site). gta-reversed inlines it as the lambda `joystickMoved`
  (MenuManager_Input.cpp:168, comment `sub_4410E0`) with `((WORD*)pad)[11] && !((WORD*)pad)[35]` - same offsets
  (0x16 / 0x46), only the name is misleading. Open questions: none.

## 0x541490 GetCurrentKeyPressed  (catalogue: specified/REA, port level 0)
- `__cdecl RsKeyCodes* GetCurrentKeyPressed(RsKeyCodes* out [esp+4])`, plain `ret`; EAX = `out` on return (loaded at
  0x541490 and never changed). gr declares `int GetCurrentKeyPressed(RsKeyCodes&)` (plugin::CallAndReturn, Pad.cpp:1424);
  the "int" is actually the pointer.
- Caller: CMenuManager::CheckRedefineControlInput 0x57E4D0. No callees.
- Globals (read): `CPad::NewKeyState` 0xB73190 and `CPad::OldKeyState` 0xB72F20 (CKeyboardState, 0x270 bytes; FKeys[12]
  at +0, standardKeys[256] at +0x18, then esc +0x218, insert +0x21A ... apps +0x26E, all i16).
- "just pressed(k)" below = `New.k != 0 && Old.k == 0` (16-bit compares).
- Behaviour (every match overwrites *out, so the LAST match in this order wins):
  1. `*out = 0x420` (1056 = rsNULL) (0x541495).
  2. Standard keys (0x5414A3-0x54152C): for i = 0 .. 254 (5 keys per iteration, loop ends when the next first index
     reaches 255; standardKeys[255] is never tested): if just pressed(standardKeys[i]) `*out = i`.
  3. F keys (0x541540-0x5415DA, 2 iterations of 6): for j = 0 .. 11: if just pressed(FKeys[j]) `*out = 0x3E9 + j`
     (rsF1 1001 .. rsF12 1012).
  4. Special keys, tested in exactly this order (offset in CKeyboardState -> value):
     esc +0x218 -> 0x3E8; insert +0x21A -> 0x3F5; del +0x21C -> 0x3F6; home +0x21E -> 0x3F7; end +0x220 -> 0x3F8;
     pgup +0x222 -> 0x3F9; pgdn +0x224 -> 0x3FA; up +0x226 -> 0x3FB; down +0x228 -> 0x3FC; left +0x22A -> 0x3FD;
     right +0x22C -> 0x3FE; scroll +0x22E -> 0x410; pause +0x230 -> 0x411; numlock +0x232 -> 0x409; div +0x234 -> 0x3FF;
     mul +0x236 -> 0x400; sub +0x238 -> 0x402; add +0x23A -> 0x401; enter +0x23C -> 0x40F; decimal +0x23E -> 0x403;
     num1 +0x240 -> 0x404; num2 +0x242 -> 0x405; num3 +0x244 -> 0x406; num4 +0x246 -> 0x407; num5 +0x248 -> 0x408;
     num6 +0x24A -> 0x40A; num7 +0x24C -> 0x40B; num8 +0x24E -> 0x40C; num9 +0x250 -> 0x40D; num0 +0x252 -> 0x40E;
     back +0x254 -> 0x412; tab +0x256 -> 0x413; capslock +0x258 -> 0x414; extenter +0x25A -> 0x415; lshift +0x25C -> 0x416;
     shift +0x260 -> 0x418 (tested BEFORE rshift, 0x541970); rshift +0x25E -> 0x417 (0x54198A); lctrl +0x262 -> 0x419;
     rctrl +0x264 -> 0x41A; lalt +0x266 -> 0x41B; ralt +0x268 -> 0x41C; lwin +0x26A -> 0x41D; rwin +0x26C -> 0x41E;
     apps +0x26E -> 0x41F (0x541A54).
- Edge cases: standardKeys[255] ignored (loop bound 0x541526 `cmp esi,0xff; jl`). With both shift and rshift newly
  pressed the result is rshift (0x417) because of the swapped test order. Open questions: none.

## 0x718630 GxtCharStrcat  (catalogue: specified/REA, port level 0)
- `__cdecl GxtChar* (GxtChar* dst [esp+4], const GxtChar* src [esp+8])`, plain `ret`, EAX = dst.
- Find the terminator of dst (0x718634-0x718646), copy src bytes up to (not including) its NUL (0x718648-0x718659), then
  write one NUL (0x71865B). Plain byte strcat, no bounds. Note: the function writes EDI nowhere (`mov edi, edi` is a
  2-byte nop). Caller: CMenuManager::DrawStandardMenus 0x5794A0. gta-reversed GxtChar.cpp:54 matches. Open questions: none.

## 0x745A70 sub_745A70 (proposed: RwEngineGetNumVideoModes thunk)  (catalogue: unknown/REA, port level 3)
- Single instruction `jmp 0x7F2CC0` (RwEngineGetNumVideoModes, `__cdecl RwInt32 ()`); behaviour = that function.
  Callers: CMenuManager::ProcessPCMenuOptions 0x57CD50 (2 sites; gr MenuManager_Process.cpp:503 calls
  RwEngineGetNumVideoModes directly). Port: forward to `orig!(0x7F2CC0)` / the RW port. Open questions: none.

## 0x746F70 DIReleaseMouse  (catalogue: specified/REA, port level 0)
- `__cdecl`, no args, plain `ret`. Global: `RsGlobal.ps` at 0xC17054 (psGlobalType*), field +0x1C = mouse
  IDirectInputDevice8*.
- Behaviour: `dev = RsGlobal.ps->mouse` (0x746F75); if null return (EAX = 0). Else `dev->Unacquire()` (vtable +0x20,
  slot 8, `push dev; call [ecx+0x20]`, 0x746F7F); re-read `RsGlobal.ps->mouse` and call `->Release()` (vtable +8,
  0x746F8E); `RsGlobal.ps->mouse = NULL` (0x746F97). EAX on return = Release's return value (ref count), which gr
  types as `IDirectInputDevice8*` (Pad.cpp:1429, plugin::CallAndReturn); callers (CMenuManager::CheckForMenuClosing
  0x576B70, 3 sites) ignore it. Open questions: none.
