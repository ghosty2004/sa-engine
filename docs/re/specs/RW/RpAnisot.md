# RenderWare 3.6 RpAnisot (texture anisotropy plugin, id 0x127) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 toolkit code in gta_sa.exe 1.0 US. gta-reversed / plugin-sdk only have call thunks, so
everything below comes from the bytes. All functions are `__cdecl` (caller pops, plain `ret`).

## Layouts used (offsets proven by the instruction addresses given)

Globals (all .data, zero-initialised):
- `[0xc92474] i32 anisotEngineOpenCount`: incremented by the engine Open callback (0x749000..0x749006).
- `[0xc92478] i32 anisotTextureOffset`: result of `RwTextureRegisterPlugin(1, 0x127, ...)` (written 0x748fae).
- `[0xc9247c] i32 anisotTextureStreamOffset`: result of `RwTextureRegisterPluginStream(0x127, ...)` (written 0x748fd6).
- `[0xc9a5e8] i32` in the D3D9 driver: copy of the texture offset, written by `_rwD3D9SetMaxAnisotropy` 0x7fde60
  (`mov [0xc9a5e8], arg` at 0x7fde64); the D3D9 texture-stage code reads the per-texture byte through it.

Texture extension: ONE byte `u8 maxAnisotropy` at `texture + [0xc92478]` (write 0x748f42, read 0x748f5e, ctor
store 0x74902a). Registered with size 1 (`push 1` at 0x748fa4); the registry rounds the size up (see plugins.md).

D3DCAPS9 (as returned by `RwD3D9GetCaps` 0x7fad20, pointer to the cached caps 0xc9bf00): +0x6c `MaxAnisotropy` (DWORD)
read at 0x748f25.

## 0x748F20 RpAnisotGetMaxSupportedMaxAnisotropy  (catalogue: specified/REA, port level 1)
- Convention: `RwInt8 (void)`, `__cdecl`, returns AL. Caller: RwTextureGtaStreamRead 0x730e60.
- Behaviour: `caps = RwD3D9GetCaps()` (0x748f20); `return (u8)caps->MaxAnisotropy` - only the LOW BYTE of the DWORD at
  caps+0x6c is loaded (`mov al, [eax+0x6c]` 0x748f25); the upper EAX bits hold the caps pointer bits (callers use AL).
- librw: absent (librw has no GetMaxSupportedMaxAnisotropy).

## 0x748F30 RpAnisotTextureSetMaxAnisotropy  (catalogue: specified/REA, port level 0)
- Convention: `RwTexture* (RwTexture* tex, RwInt8 value)`; value is the byte at [esp+8] (0x748f3e).
- Behaviour: `if (tex == NULL) return NULL` (0x748f34/0x748f46); `*(u8*)(tex + [0xc92478]) = value` (0x748f42);
  return tex (EAX still holds tex). No check that the plugin was registered (offset may be -1 if the attach failed).
- librw: texture.cpp:Texture::setMaxAnisotropy differs: librw stores an int32 and skips the store when the offset is <= 0;
  the exe stores one byte and tests the texture pointer instead.

## 0x748F50 RpAnisotTextureGetMaxAnisotropy  (catalogue: specified/REA, port level 0)
- Convention: `RwInt8 (RwTexture* tex)`, returns AL.
- Behaviour: `tex == NULL` -> return 0 (`xor al, al` 0x748f62); else return `*(u8*)(tex + [0xc92478])` (0x748f5e).
  Upper bits of EAX = tex bits (only AL is defined).
- librw: texture.cpp:Texture::getMaxAnisotropy differs: int32 field, offset check instead of NULL check.

## 0x748F70 RpAnisotPluginAttach  (catalogue: specified/REA, port level 3)
- Convention: `RwBool (void)`; returns 1 or 0. Caller: App::PluginAttach 0x53d870 (return value ignored there).
- Callees: RwEngineRegisterPlugin 0x7f2bb0, RwTextureRegisterPlugin 0x7f3bb0, RwTextureRegisterPluginStream 0x804550,
  _rwD3D9SetMaxAnisotropy 0x7fde60.
- Behaviour, in order:
  1. `RwEngineRegisterPlugin(0, 0x127, Open=0x749000, Close=0x749010)` (0x748f70..0x748f81); `< 0` -> return 0 (0x748f8b).
  2. `[0xc92478] = RwTextureRegisterPlugin(1, 0x127, Ctor=0x749020, Dtor=0x749030, Copy=0x749040)` (0x748f90..0x748fae);
     `< 0` -> return 0 (0x748fb5; the offset global keeps the negative value).
  3. `[0xc9247c] = RwTextureRegisterPluginStream(0x127, Read=0x749060, Write=0x7490b0, GetSize=0x7490f0)` (0x748fba..0x748fd6);
     `< 0` -> return 0 (0x748fdd).
  4. `_rwD3D9SetMaxAnisotropy([0xc92478])` (0x748fe2..0x748fe8) - hands the texture extension offset to the driver.
  5. return 1 (0x748ff0).
- Other callbacks registered here (not in this work list): Close 0x749010, Copy 0x749040 (`dst[off] = src[off]`, one byte,
  returns dst), StreamRead 0x749060 (only when `length == 4` and object != NULL: `RwStreamReadInt32(stream, &tmp, 4)`;
  failure -> NULL; stores the LOW BYTE of tmp at `object + [0xc92478]` 0x749097; returns stream).
- librw: texture.cpp:registerAnisotropyPlugin differs: no engine plugin, ext size 4 (int32), no driver offset hand-off.

## 0x749000 cb_ANISOTPLUGIN_RwEngine_Open  (catalogue: specified/REA, port level 0)
- Convention: `void* (void* instance, RwInt32 offset, RwInt32 size)`.
- Behaviour: `[0xc92474] += 1` (0x749000..0x749006); return instance ([esp+4], 0x74900b). Nothing else.
- librw: absent (no engine part).

## 0x749020 cb_ANISOTPLUGIN_RwTexture_Ctor  (catalogue: specified/REA, port level 0)
- Convention: `void* (void* texture, RwInt32 offset, RwInt32 size)`; uses the GLOBAL `[0xc92478]`, not the offset arg.
- Behaviour: `*(u8*)(texture + [0xc92478]) = 1` (0x74902a); return texture.
- librw: texture.cpp:createAnisot matches (value 1), except the int32 width.

## 0x749030 cb_ANISOTPLUGIN_RwTexture_Dtor  (catalogue: specified/REA, port level 0)
- Convention: as the ctor. Behaviour: returns texture ([esp+4], 0x749030) and does nothing else.
- librw: absent (librw registers no destructor).

## Open questions
- None.
