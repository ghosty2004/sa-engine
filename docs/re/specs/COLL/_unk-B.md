# COLL: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x59F0C0 tColLighting::GetCurrentLighting  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl float (tColLighting light [esp+4] (byte, passed by value), float scale [esp+8])`, result in ST0, plain `ret`. (gr declares a const member; the original passes the byte on the stack, no ECX.) Callers: CTrain::ProcessControl (2), CBulletInfo::Update (2), CWeapon::DoBulletImpact (3).
- Globals: CCustomBuildingDNPipeline::m_fDNBalanceParam 0x8D12C0 (f32, read first). Constants: `[0x863E0C]=0.06666667f` (1/15, f32), `[0x858624]=1.0f`.
- Behaviour (x87 order): `bal = m_fDNBalanceParam`; `day = (float)(light & 0xF)` (fild of a zero-extended int); `d = day * scale * 0.06666667f`; `d *= (1.0f - bal)`; `night = (float)(light >> 4)`; `n = night * scale * 0.06666667f * bal`; `return d + n`.
- gta-reversed: gr ColPoint.cpp:7 computes `day*scale/15` (division, not multiplication by 0.0666667) and `lerp`; the bytes multiply by the f32 constant and form `d*(1-bal) + n*bal` in this order. Open questions: none.
