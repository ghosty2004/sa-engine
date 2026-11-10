# CAEStreamingDecoder (AUDIO) - full specs (RE-SPEC-3, group U1)

Layout: +0 vptr (base vtable 0x85F418), +4 CAEDataStream* m_dataStream.

## 0x4F2860 CAEStreamingDecoder::~CAEStreamingDecoder (scalar deleting destructor)  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CAEStreamingDecoder* (this = ECX, u32 flags [esp+4])`, returns this, `ret 4`. No direct callers
  (vtable slot 0 of the abstract base 0x85F418; derived decoders have their own deleting dtors).
- Behaviour: `s = +4`; `+0 = 0x85F418`; if `s != 0`: `CAEDataStream::Destructor(s)` 0x4DC490 (thiscall), `_free(s)` 0x8214BD,
  `+4 = 0` (0x4F2864-0x4F2881). Then if `flags & 1`: `_free(this)`. Return this.
- gta-reversed: `~CAEStreamingDecoder() { delete m_dataStream; }` equivalent (the DataStream's own destructor is called
  non-virtually, then freed). Open questions: none.
