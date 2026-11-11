# CModelInfoAccelerator (STREAM) - RE-SPEC-FINAL group A

## 0x4C6B10 CModelInfoAccelerator::FreeModelInfoIds  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`: `operator delete(this->m_pIDs (+0))` 0x8214BD (CRT, null allowed); `+0 = 0`. Plain `ret`.
- gta-reversed: ModelInfoAccelerator.cpp - same. Open questions: none.

## 0x4C6B70 CModelInfoAccelerator::GetModelInfoIdFile  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall bool (this)`, plain `ret`, AL = m_bFileRead.
- Behaviour: `f = CFileMgr::OpenFile(this->m_szFilePath (+6), "rb" [0x859C6C])` 0x538900; `m_bFileRead (+0x1B) = (int32)f > 0` (`test; setg`:
  a SIGNED test of the handle); inlined AllocModelInfoIds: `p = operator new(0xA08C)` 0x82119A, `+0 = p`, zero 0x2823 dwords (`rep stosd`);
  if m_bFileRead: `CFileMgr::Read(f, p, 0xA08C)` 0x538950, `CFileMgr::CloseFile(f)` 0x5389D0. Return m_bFileRead.
- gta-reversed: ModelInfoAccelerator.cpp:61 - same except `file != nullptr` vs the original's signed `> 0` (differs only for handles with the top
  bit set). Open questions: none.

## 0x4C6BD0 CModelInfoAccelerator::EndOfLoadPhase  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (this)`, plain `ret`.
- Behaviour: if `!m_bFileRead (+0x1B)`: `f = CFileMgr::OpenFileForWriting(m_szFilePath (+6))` 0x538910; if `(int32)f > 0` (`test; jle`):
  `CFileMgr::Write(f, m_pIDs, 0xA08C)` 0x538970, `CloseFile(f)` 0x5389D0. Then (always) `operator delete(m_pIDs)` 0x8214BD, `+0 = 0`.
- gr-deviations row (ModelInfoAccelerator.cpp:81 "FIX_BUGS"): gr adds an else-branch that logs and calls `CFileMgr::CloseFile(file)` with a null
  handle when the open failed. Original bytes 0x4C6BE9-0x4C6BEB skip straight to the delete. Verdict: CONFIRMED. Class: keep-original (harmless
  for the result; do not call CloseFile on failure).
- Open questions: none.
