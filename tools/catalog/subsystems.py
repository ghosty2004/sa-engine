#!/usr/bin/env python3
"""Assign every catalogued function to a subsystem and write catalog/subsystems.md.

Rules match the gta-reversed category (prefix) first, then the owner class
(regex). Unknown/unnamed functions are bucketed by address region until CAT-5
names them.

Usage: subsystems.py <catalog-dir>
"""
import csv
import re
import sys
from collections import defaultdict
from pathlib import Path

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
    out_rows = []
    counts = defaultdict(lambda: defaultdict(int))
    for p in csv.DictReader(open(cat / 'procedures.csv')):
        if p['region'] not in ('game-text', 'rw-cseg'):
            continue
        va = int(p['address'], 16)
        h = hooks.get(va)
        if h:
            sid = classify(h['category'], h['owner'], h['name'])
        elif p['state'] == 'gr-called-only':
            sid = 'CALLED-ONLY'
        else:
            sid = next((b for lo, hi, b in UNKNOWN_BUCKETS if lo <= va < hi), 'UNK-game')
        counts[sid][p['state']] += 1
        out_rows.append({**p, 'subsystem': sid})
    with open(cat / 'procedures.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, list(out_rows[0]))
        w.writeheader()
        w.writerows(out_rows)
    states = ['gr-reversed', 'gr-unreversed', 'gr-called-only', 'unknown']
    with open(cat / 'subsystems.md', 'w') as f:
        f.write('# Functions per subsystem (generated by tools/catalog/subsystems.py)\n\n')
        f.write('| Subsystem | ' + ' | '.join(states) + ' | total |\n|---|' + '---|' * 5 + '\n')
        for sid in sorted(counts, key=lambda s: -sum(counts[s].values())):
            n = [counts[sid][s] for s in states]
            f.write(f'| {sid} | ' + ' | '.join(map(str, n)) + f' | {sum(n)} |\n')
        tot = [sum(counts[s][st] for s in counts) for st in states]
        f.write('| **total** | ' + ' | '.join(map(str, tot)) + f' | {sum(tot)} |\n')
    print(open(cat / 'subsystems.md').read())


if __name__ == '__main__':
    main()
