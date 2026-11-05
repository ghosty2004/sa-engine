# INPUT static initialisers / atexit destructors

## 0x84E1F0 __init_CPad__Pads  (catalogue: specified/REA, port level 0)
- `__cdecl void()` CRT initialiser. `eh_vector_constructor_iterator(0xB73458 /*CPad::Pads*/, 0x134 /*size*/, 2 /*count*/,
  CPad::CPad 0x541D80, CPad::~CPad 0x53ED60)` (0x84E206), then `_atexit(__dtor_CPad__Pads 0x856460)` (0x84E210); `pop ecx; ret`.

## 0x856460 __dtor_CPad__Pads  (catalogue: specified/REA, port level 0)
- `__cdecl void()` atexit callback. `eh_vector_destructor_iterator(0xB73458, 0x134, 2, CPad::~CPad 0x53ED60)` (0x856471).
  The element destructor is an empty `ret`, so this has no effect.

## 0x84D920 __init_ControlsManager  (catalogue: specified/REA, port level 4)
- `__cdecl void()`: `mov ecx, 0xB70198 /*ControlsManager*/; jmp CControllerConfigManager::CControllerConfigManager 0x531EE0`
  (tail call; no atexit registration, the class has no destructor registered).
