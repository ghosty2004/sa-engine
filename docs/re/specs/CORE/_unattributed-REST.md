# _unattributed-REST (CORE) — RE-SPEC-REST

## 0x746480 sub_746480 (proposed: `CommandLineToArgv(const char* cmdLine /*EAX*/, int32* argc)`)  (catalogue: unknown/REA, port level 0)
- Non-standard convention: the command-line string is passed in **EAX**; `int32* argc` at [esp+4]; caller cleans (plain `ret`);
  returns EAX = `char** argv` (one malloc block). Caller: Win::NOTSA_WinMain 0x748710 (the PC WinMain). Callee: `malloc` 0x824257.
- Behaviour:
  1. `len = strlen(s)`. Pass 1 (0x7464B0, i = 0 .. len inclusive, so the terminating NUL is visited): `quoted` toggles on every `"`;
     a character is a separator when `(c <= 0x20 (unsigned) && !quoted) || i == len`; on a separator, if inside a token, `count++` and
     leave the token; on a non-separator, enter the token (a `"` also enters a token here).
  2. `block = malloc(len + count*4 + 1)`; `S = block + count*4`; `strcpy(S, s)`.
  3. Pass 2 (0x746521, same scan over s, separate `quoted` flag): on a separator while inside a token: if `S[i-1] == '"'` then
     `S[i-1] = 0` (the closing quote is cut) else `S[i] = 0`; leave the token. On a non-separator while outside a token and `c != '"'`:
     `argv[k++] = &S[i]`; enter the token (a leading quote is skipped: the token starts at the next character).
  4. `*argc = count` (from pass 1); return block.
- ORIGINAL BUG (keep): pass 1 counts a token made only of quotes (e.g. `""`), pass 2 never stores a pointer for it, so `argc` can exceed
  the number of initialised argv entries (the extra slots are uninitialised malloc memory). No argv[argc] = NULL terminator is written.
