# RW: functions without gta-reversed source (RE-SPEC-UNK lead)

IJG libjpeg 6b (JPEG_LIB_VERSION 62: `jpeg_CreateCompress` 0x5CF960 compares the version with 0x3E and the
struct size with 0x168) is statically linked at 0x5C26B0-0x5D0260 and proposed as
`excluded:thirdparty:libjpeg-6b` (see catalog proposal). The functions below are the game-written glue
around it: the libjpeg memory back end (IJG `jmemnobs.c` with malloc/free replaced by CMemoryMgr) and the
destination manager of JPegPlugin::JPegCompressScreenToBuffer (0x5D0740). All are never executed in the
SA:MP coverage corpus (screenshots only).

## 0x5cf800 jpeg_get_small (proposed, jmemnobs)  (catalogue: unknown/REA, port level 1)
- `__cdecl void* (j_common_ptr cinfo [esp+4], size_t sizeofobject [esp+8])`, plain `ret`.
- 0x5CF800-0x5CF80F: push 0; push sizeofobject; call 0x72F420 CMemoryMgr::Malloc(size, 0); `add esp, 8`;
  returns its EAX. cinfo is unused. Callers: libjpeg jmemmgr (2 sites).
- Name from IJG source order (get_small, free_small, get_large, free_large) and the argument shapes: medium.

## 0x5cf810 jpeg_free_small (proposed, jmemnobs)  (catalogue: unknown/REA, port level 1)
- `__cdecl void (j_common_ptr cinfo [esp+4], void* object [esp+8], size_t size [esp+0xC])`.
- 0x5CF810-0x5CF81B: push object; call 0x72F430 CMemoryMgr::Free; `pop ecx`; `ret`. cinfo and size unused.

## 0x5cf820 jpeg_get_large (proposed, jmemnobs)  (catalogue: unknown/REA, port level 1)
- Same body as 0x5CF800: returns CMemoryMgr::Malloc(sizeofobject [esp+8], 0) (0x5CF827).

## 0x5cf830 jpeg_free_large (proposed, jmemnobs)  (catalogue: unknown/REA, port level 1)
- Same body as 0x5CF810: CMemoryMgr::Free(object [esp+8]) (0x5CF835).

## 0x5d0260 JPegPlugin::JPegCompressScreenToBuffer::init_destination (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl void (j_compress_ptr cinfo [esp+4])`; referenced only as an immediate stored into the local
  `jpeg_destination_mgr` by 0x5D0740 (0x5D076D).
- 0x5D0264: ECX = cinfo->dest ([cinfo+0x18]); dword [dest+0] (next_output_byte) = 0xBD0B78
  (g_ScreenshotFileBuf, gr StaticRef, 204800 bytes); 0x5D0270: dword [dest+4] (free_in_buffer) = 0x32000.
- gta-reversed: lambda in JPegCompress.cpp (same behaviour).

## 0x5d0280 JPegPlugin::JPegCompressScreenToBuffer::empty_output_buffer (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl boolean (j_compress_ptr cinfo [esp+4])`; returns AL = 1 (0x5D02CC `mov al, 1`; upper EAX bits
  are the dest pointer, not cleared). Referenced as an immediate at 0x5D0775.
- Globals: 0xC02B78 Buffer (u8**), 0xC02B7C BufferSize (u32), 0xC02B80 TotalProcessed (u32), 0xBD0B78 buffer.
- Behaviour: EAX = TotalProcessed; ESI = BufferSize; ECX = TotalProcessed + 0x32000; EDX = 0x32000; if
  (ECX >= ESI, unsigned `jae` at 0x5D0299) skip the copy; else copy 0xC800 dwords (`rep movsd`) from 0xBD0B78
  to *Buffer + TotalProcessed and TotalProcessed += 0x32000 (0x5D02B2). Then (always) dest->next_output_byte
  = 0xBD0B78, dest->free_in_buffer = 0x32000 (EDX), return 1. A full buffer that does not fit is silently
  dropped and the output continues (original behaviour).
- gta-reversed: lambda, same logic.

## 0x5d02d0 JPegPlugin::JPegCompressScreenToBuffer::term_destination (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl void (j_compress_ptr cinfo [esp+4])`, plain `ret`. Referenced as an immediate at 0x5D077D.
- Behaviour: ECX = used = dest->next_output_byte - 0xBD0B78 (0x5D02DE); EAX = TotalProcessed; if
  (TotalProcessed + used >= BufferSize, unsigned `jae` at 0x5D02F0) return; else copy `used` bytes to
  *Buffer + TotalProcessed (`rep movsd` of used>>2 dwords, then `rep movsb` of used&3 bytes) and
  TotalProcessed += 0x32000 (0x5D0310: the FULL buffer size, not `used`; original bug — the caller
  0x5D0740 then reports TotalProcessed as the output size, so the size is rounded up to a multiple of 0x32000).
- gta-reversed deviates: its lambda copies `g_ScreenshotFileBuf.size()` (0x32000) bytes instead of `used`
  (the bytes copy only `used`); both add the full size. Port follows the bytes.
