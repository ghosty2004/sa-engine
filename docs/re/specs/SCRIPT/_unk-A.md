# SCRIPT: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x463750 tScriptText element ctor (trivial) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = this; nothing initialised (0x463750). Element ctor passed to the vector constructor of CTheScripts::IntroTextLines.

## 0x463760 tScriptText element dtor (empty) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret`.

## 0x463770 tScriptRectangle element ctor (trivial) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = this; nothing initialised (0x463770). Element ctor for CTheScripts::IntroRectangles.

## 0x463780 tScriptRectangle element dtor (empty) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret`.

## 0x46ae30 struct ctor (clears bytes +0 and +0x18) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`: `byte[+0] = 0; byte[+0x18] = 0`, EAX = this (0x46ae32/0x46ae35).
- Open question: owner type; referenced as a function pointer from 0x66d440 only.

## 0x46b260 CScriptsForBrains::CScriptsForBrains (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall CScriptsForBrains* ()`: `this->Init()` 0x46a8c0; EAX = this.
