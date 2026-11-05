#!/usr/bin/env python3
"""Assign every catalogued function to a subsystem and write catalog/subsystems.md.

Rules match the gta-reversed category (prefix) first, then the owner class
(regex). Unknown functions take their class and subsystem from
catalog/classification.csv (CAT-5, tools/catalog/classify_unknowns.py): excluded
rows get state `excluded` and subsystem `X-<kind>`, RW rows `RW`, game rows the
subsystem found there (or UNK-game). Without that file they fall back to the
address-region buckets below.

Usage: subsystems.py <catalog-dir>
"""
import csv
import re
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

# (subsystem id, category prefixes, owner regex)
RULES = [
    ('MATH', [], r'^(CVector\w*|CMatrix\w*|CQuaternion|CRect|CBox|CSphere|CColSphere|CColBox|CColLine|CCompressed\w*|CMaths|CGeneral)$'),
    ('CORE', ['Core', 'Common', 'App', 'Win', 'Pools', 'Memory'],
     r'^(CTimer|CPools|CPool\w*|CMemoryMgr|CFileMgr|CDirectory|CIniFile|CGame|VideoMode|Rs|CDebug|CLoadMonitor|CReferences|Securom|CKeyGen)$'),
    ('STREAM', ['Models'], r'^(CStreaming|CStreamingInfo|CdStream|CFileLoader|CIplStore|CTxdStore|CColStore|CModelInfo|C\w*ModelInfo|CLoadedCarGroup|ModelIndices|CLoadingScreen)$'),
    ('RW', ['RenderWare', 'Renderware', 'Plugins', 'Pipelines'],
     r'^(RwHelper|CVisibilityPlugins|CCustomBuilding\w*|CCustomCarEnvMapPipeline|RenderBuffer|CCarFXRenderer|CClothesBuilder)$'),
    ('COLL', ['Collision'], r'^(CCollision|SurfaceInfos_c|CColModel|CCollisionData|CColTriangle\w*|CColPoint)$'),
    ('PHYS', [], r'^(CPhysical|cBuoyancy)$'),
    ('ANIM', ['Animation'], r'^(IKChain\w*|BoneNode\w*|CPedIK|CVehicleAnimGroup\w*|CAnim\w*|CCutsceneMgr)$'),
    ('AUDIO', ['Audio'], r'^(CAE\w*|CAudio\w*)$'),
    ('AI', ['Tasks', 'Events', 'DecisionMakers', 'Attractors', 'AI', 'Conversations'],
     r'^(CPedIntelligence|CPedAttractor\w*|CPedGeometryAnalyser|CCover\w*|CEntityScanner|CPedGroup\w*|CFormation|CInform\w*|CAcquaintance|CInterestingEvents|CPedTaskPair|CTask\w*|CEvent\w*|CDecisionMaker\w*|CPedList|CConversations?|CPedScriptedTaskRecord\w*)$'),
    ('POP', [], r'^(CPopulation|CPopCycle|CCarCtrl|CCarAI|CCarGenerator|CTheCarGenerators|CRoadBlocks|CGangs|CGangWars|CAccidentManager|CTagManager|CTrafficLights|CPathFind|CPathNode|CAutoPilot|CDarkel)$'),
    ('VEH', ['Vehicle'], r'^(CDamageManager|cHandlingDataMgr|cTransmission|CCarEnterExit|CVehicleRecording|CSpecialPlateHandler|CUpsideDownCarCheck|CStuckCarCheck|CCurrentVehicle|CDoor|CBouncingPanel|CVehicle\w*|CAutomobile|CBike|CBmx|CBoat|CHeli|CPlane|CTrain|CTrailer|CQuadBike|CMonsterTruck)$'),
    ('PED', ['Entity/Ped', 'Entity/CivilianPed'],
     r'^(CPed|CPlayerPed|CCopPed|CEmergencyPed|CCivilianPed|CPlayerInfo|CPlayerPedData|CPedType|CPedStats|CPedClothesDesc|CClothes|CPedDamageResponse\w*|CPedPlacement|CWanted|CCrime)$'),
    ('WEAPON', [], r'^(CWeapon|CWeaponInfo|CProjectile\w*|CBulletInfo|CShotInfo|CExplosion|CFire|CFireManager|CCreepingFire|CWaterCannons?|CWeaponEffects)$'),
    ('CAM', ['Camera'], r'^(CCamera|CCam|CIdleCam|CHandShaker|CCurves|CCamPathSplines)$'),
    ('INPUT', [], r'^(CPad|CControllerConfigManager|CMouseControllerState|CControllerState|CKeyboardState)$'),
    ('HUD', ['Text'], r'^(CHud|CRadar|CFont|CMessages|CMenuManager|CMenuSystem|CText|CLocalisation|COnscreen\w*|CHudColours|CUserDisplay|CPlaceName|CCredits)$'),
    ('RENDER', ['Fx', 'Shadows', 'Plant'],
     r'^(CRenderer|CSprite2?d?|CCoronas|CRegisteredCorona|CShadows|CStencilShadow\w*|CPostEffects|CClouds|CTimeCycle|CColourSet|CMirrors|CPointLights|CSpecialFX|CBrightLights|CShinyTexts|CMotionBlurStreaks|CRegisteredMotionBlurStreak|CPlaneTrails?|CSkidmarks|CBulletTraces|C3dMarkers?|CCheckpoints?|COcclusion|COccluder|CActiveOccluder|CCullZones|CGamma|CDraw|CLines|CGlass|CFallingGlassPane|CCustomRoadsignMgr|CCustomCarPlateMgr|CBirds|CMovingThings|CRopes?|CWaterLevel|CWeather|CClock|CWindModifiers)$'),
    ('SCRIPT', ['Scripts'], r'^(CTheScripts|CRunningScript|CScripted2dEffects|CScriptsForBrains|CScriptResourceManager|CStreamedScripts|CMissionCleanup|CUpsideDownCarCheck)$'),
    ('SAVE', [], r'^(CGenericGameStorage|C_PcSave|\w*SaveStructure|CPedSaveStructure)$'),
    ('WORLD', ['Entity', 'Interior'],
     r'^(CWorld|CEntity|CObject|CDummy\w*|CBuilding|CTheZones|CZone\w*|CGridRef|CEntryExit\w*|Interior\w*|CBridge|CCranes?|CEscalators?|BreakObject_c|BreakManager_c|CGarages?|CStuntJumpManager|CPickups?|CSetPieces?|CObjectData|WaterCreature\w*|CTreadable|CReplay|CGameLogic|CStats|CRestart|CCheat|CShopping|CGangWars)$'),
]
COMPILED = [(sid, cats, re.compile(rx)) for sid, cats, rx in RULES]
NAME_COLS = ('name', 'name_source', 'name_confidence', 'role', 'name_hint')  # CAT-6, written here
UNKNOWN_BUCKETS = [  # address-region buckets for unnamed game-.text functions (CAT-5)
    (0x400000, 0x410000, 'UNK-protection-mix'),
    (0x760000, 0x7C0000, 'UNK-thirdparty-lib'),
    (0x7C0000, 0x830000, 'UNK-rw-crt'),
    (0x830000, 0x850000, 'UNK-eh-funclets'),
]


def classify(category, owner, name):
    for sid, cats, rx in COMPILED:
        if owner and rx.match(owner):
            return sid
    for sid, cats, rx in COMPILED:
        if any(category.startswith(c) for c in cats):
            return sid
    if name.startswith(('Rw', 'Rp', 'Rt', 'Rx', '_rw', '_rp')):
        return 'RW'
    return 'OTHER'


def main():
    cat = Path(sys.argv[1])
    hooks = {}
    for h in csv.DictReader(open(cat / 'functions.csv')):
        hooks[int(h['address'], 16)] = h
    cls = {}
    if (cat / 'classification.csv').exists():
        for c in csv.DictReader(open(cat / 'classification.csv')):
            cls[int(c['address'], 16)] = c
    import names_extra as NX  # noqa: E402  (CAT-6 name precedence; imported late: it imports this module)
    rwn = NX.load_rw_names(cat)
    extra = NX.load_names_extra(cat)
    out_rows = []
    counts = defaultdict(lambda: defaultdict(int))
    for p in csv.DictReader(open(cat / 'procedures.csv')):
        if p['region'] not in ('game-text', 'rw-cseg'):
            continue
        for k in NAME_COLS:
            p.pop(k, None)
        va = int(p['address'], 16)
        h = hooks.get(va)
        if h:
            sid = classify(h['category'], h['owner'], h['name'])
        elif p['state'] == 'gr-called-only':
            sid = 'CALLED-ONLY'
        elif va in cls:
            c = cls[va]['class']
            if c.startswith('excluded:'):
                p['state'] = 'excluded'
                sid = 'X-' + c.split(':', 1)[1].split(':')[-1]
            elif c == 'RW':
                sid = 'RW'
            else:
                sid = cls[va]['subsystem'] or 'UNK-game'
        else:
            sid = next((b for lo, hi, b in UNKNOWN_BUCKETS if lo <= va < hi), 'UNK-game')
        name, nsrc, nconf, hint = NX.final_name(p, cls.get(va), rwn.get(va), extra.get(va))
        x, r = extra.get(va), rwn.get(va)
        if p['state'] != 'excluded' and sid in ('CALLED-ONLY', 'UNK-game'):
            # CAT-6: a RenderWare name puts a called-only RW function into RW; a names-extra name
            # brings its subsystem (owner class, or the global an initialiser constructs)
            if nsrc == 'rw-names' and r['module'] not in ('game-plugin', 'game-pipeline'):
                sid = 'RW'
            elif nsrc.startswith('cat6:') and x['subsystem']:
                sid = x['subsystem']
            elif nsrc and nsrc != 'gta-reversed' and name:
                o = name.split('::')[0] if '::' in name else ''
                sid = next((s_ for s_, _c, rx in COMPILED if o and rx.match(o)), sid)
        counts[sid][p['state']] += 1
        out_rows.append({**p, 'subsystem': sid, 'class': cls[va]['class'] if va in cls else '',
                         'name': name, 'name_source': nsrc, 'name_confidence': nconf,
                         'role': x['role'] if x and nsrc.startswith('cat6:') else '', 'name_hint': hint})
    with open(cat / 'procedures.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, list(out_rows[0]))
        w.writeheader()
        w.writerows(out_rows)
    states = ['gr-reversed', 'gr-unreversed', 'gr-called-only', 'unknown', 'excluded']
    with open(cat / 'subsystems.md', 'w') as f:
        f.write('# Functions per subsystem (generated by tools/catalog/subsystems.py)\n\n')
        f.write('| Subsystem | ' + ' | '.join(states) + ' | total |\n|---|' + '---|' * (len(states) + 1) + '\n')
        for sid in sorted(counts, key=lambda s: -sum(counts[s].values())):
            n = [counts[sid][s] for s in states]
            f.write(f'| {sid} | ' + ' | '.join(map(str, n)) + f' | {sum(n)} |\n')
        tot = [sum(counts[s][st] for s in counts) for st in states]
        f.write('| **total** | ' + ' | '.join(map(str, tot)) + f' | {sum(tot)} |\n')
    reconcile(cat, out_rows, counts, states)
    print(open(cat / 'subsystems.md').read())


def reconcile(cat, rows, counts, states):
    """CAT-9: one count per function across summary.md, subsystems.md and procedures.csv."""
    first = {}
    for h in csv.DictReader(open(cat / 'functions.csv')):
        first.setdefault(int(h['address'], 16), h)
    inrows = {int(r['address'], 16): r for r in rows}
    total = sum(sum(c.values()) for c in counts.values())
    assert total == len(rows), (total, len(rows))
    hooked = [a for a in first if a in inrows]
    missing = sorted(a for a in first if a not in inrows)
    stubs = [a for a in hooked if first[a]['status'] == 'stub-to-.HOODLUM']
    gr = {st: sum(1 for a in hooked if inrows[a]['state'] == st) for st in ('gr-reversed', 'gr-unreversed')}
    lines = [
        '\n## Reconciliation (CAT-9, generated by subsystems.py)\n',
        f'- `procedures.csv` rows = `subsystems.md` total = **{len(rows)}** functions (game .text + '
        f'_rwcseg). Each function is one row; a `.HOODLUM` body is part of its stub\'s row.',
        f'- Distinct gta-reversed hook addresses: {len(first)}. In the catalogue: {len(hooked)} '
        f'(gr-reversed {gr["gr-reversed"]}, gr-unreversed {gr["gr-unreversed"]}; {len(stubs)} of them '
        f'are `.HOODLUM` stubs, counted once at the stub). Not in the catalogue: {len(missing)} ('
        + ', '.join(f'{a:#x} {first[a]["owner"]}::{first[a]["name"]} {first[a]["status"]}' for a in missing)
        + ').',
        f'- The registration counts at the top (with duplicates, first registration of an address '
        f'decides its reversed flag here) are source counts, not function counts.',
    ]
    text = open(cat / 'summary.md').read().split('\n## Reconciliation (CAT-9')[0]
    with open(cat / 'summary.md', 'w') as f:
        f.write(text + '\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
