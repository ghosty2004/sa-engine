# Catalogue summary (generated)

- Source: `re/gta-reversed`
- Binary: `re/gta/1.0-US/gta_sa.exe`

- Hook registrations: 8635 (8632 distinct addresses)
- Marked reversed: 7904, not reversed: 731
- Addresses registered more than once: 3
- Callee addresses into the original (plugin::Call / raw thunk): 1340 distinct, 1343 sites, 944 not hooked anywhere

## Address verification against 1.0 US

| Status | Count |
|---|---|
| ok | 8254 |
| stub-to-.HOODLUM | 379 |
| outside-image | 1 |
| outside-game-text | 1 |

## Non-ok hook addresses (first 200)

| Address | Owner::Name | Status | First insn |
|---|---|---|---|
| 0xdeadbeef | CPedAttractor::GetNoOfRegisteredPeds | outside-image | `` |
| 0x41c4a0 | CCarAI::AddAmbulanceOccupants | stub-to-.HOODLUM | `jmp 0x1563ce0` |
| 0x41c600 | CCarAI::AddFiretruckOccupants | stub-to-.HOODLUM | `jmp 0x1568c10` |
| 0x41bfa0 | CCarAI::BackToCruisingIfNoWantedLevel | stub-to-.HOODLUM | `jmp 0x15612e0` |
| 0x41c050 | CCarAI::CarHasReasonToStop | stub-to-.HOODLUM | `jmp 0x156fb90` |
| 0x41ca50 | CCarAI::FindPoliceBoatMissionForWantedLevel | stub-to-.HOODLUM | `jmp 0x156ffb0` |
| 0x41c760 | CCarAI::TellOccupantsToLeaveCar | stub-to-.HOODLUM | `jmp 0x1569a30` |
| 0x421740 | CCarCtrl::InitSequence | stub-to-.HOODLUM | `jmp 0x15616e0` |
| 0x42c250 | CCarCtrl::IsAnyoneParking | stub-to-.HOODLUM | `jmp 0x15636d0` |
| 0x423ed0 | CCarCtrl::RemoveFromInterestingVehicleList | stub-to-.HOODLUM | `jmp 0x1561040` |
| 0x42fbc0 | CCarCtrl::ScriptGenerateOneEmergencyServicesCar | stub-to-.HOODLUM | `jmp 0x1570100` |
| 0x423f00 | CCarCtrl::ClearInterestingVehicleList | stub-to-.HOODLUM | `jmp 0x15649a0` |
| 0x421770 | CCarCtrl::FindSequenceElement | stub-to-.HOODLUM | `jmp 0x1565130` |
| 0x42a730 | CCarCtrl::GetAIHeliToFlyInDirection | stub-to-.HOODLUM | `jmp 0x156dd90` |
| 0x42f870 | CCarCtrl::JoinCarWithRoadSystemGotoCoors | stub-to-.HOODLUM | `jmp 0x156a4a0` |
| 0x423880 | CCarCtrl::SteerAIPlaneToCrashAndBurn | stub-to-.HOODLUM | `jmp 0x156a3a0` |
| 0x422590 | CCarCtrl::StopCarIfNodesAreInvalid | stub-to-.HOODLUM | `jmp 0x1567ad0` |
| 0x423fc0 | CCarCtrl::SwitchVehicleToRealPhysics | stub-to-.HOODLUM | `jmp 0x1569420` |
| 0x4067b0 | CdStream::CdStreamOpen | stub-to-.HOODLUM | `jmp 0x1564a90` |
| 0x406460 | CdStream::CdStreamSync | stub-to-.HOODLUM | `jmp 0x156cd80` |
| 0x4063e0 | CdStream::CdStreamGetStatus | stub-to-.HOODLUM | `jmp 0x1563850` |
| 0x406a20 | CdStream::CdStreamRead | stub-to-.HOODLUM | `jmp 0x156c2c0` |
| 0x439af0 | CCheat::DoCheats | stub-to-.HOODLUM | `jmp 0x1566d70` |
| 0x438450 | CCheat::ResetCheats | stub-to-.HOODLUM | `jmp 0x1560f40` |
| 0x407410 | CCheat::IsZoneStreamingAllowed | stub-to-.HOODLUM | `jmp 0x1563a60` |
| 0x40ede0 | CBox::Set | stub-to-.HOODLUM | `jmp 0x1563af0` |
| 0x40ef10 | CColLine::Set | stub-to-.HOODLUM | `jmp 0x156d710` |
| 0x40fc40 | CColModel::operator delete | stub-to-.HOODLUM | `jmp 0x1570190` |
| 0x40f7c0 | CColModel::operator= | stub-to-.HOODLUM | `jmp 0x15697f0` |
| 0x40f740 | CColModel::MakeMultipleAlloc | stub-to-.HOODLUM | `jmp 0x1564a10` |
| 0x40f810 | CColModel::AllocateData-void | stub-to-.HOODLUM | `jmp 0x156deb0` |
| 0x40f870 | CColModel::AllocateData-params | stub-to-.HOODLUM | `jmp 0x1561730` |
| 0x4103a0 | CColStore::AddCollisionNeededAtPosn | stub-to-.HOODLUM | `jmp 0x15672b0` |
| 0x411140 | CColStore::AddColSlot | stub-to-.HOODLUM | `jmp 0x1563290` |
| 0x4107a0 | CColStore::AddRef | stub-to-.HOODLUM | `jmp 0x1569960` |
| 0x4107d0 | CColStore::RemoveRef | stub-to-.HOODLUM | `jmp 0x156dbe0` |
| 0x4104e0 | CColStore::SetCollisionRequired | stub-to-.HOODLUM | `jmp 0x156cf50` |
| 0x411330 | CColStore::RemoveColSlot | stub-to-.HOODLUM | `jmp 0x1567210` |
| 0x410730 | CColStore::RemoveCol | stub-to-.HOODLUM | `jmp 0x1564e90` |
| 0x4106d0 | CColStore::LoadCol-2 | stub-to-.HOODLUM | `jmp 0x15613a0` |
| 0x410820 | CColStore::IncludeModelIndex | stub-to-.HOODLUM | `jmp 0x156a260` |
| 0x410800 | CColStore::GetBoundingBox | stub-to-.HOODLUM | `jmp 0x1565240` |
| 0x417730 | CCollision::TestLineOfSight | stub-to-.HOODLUM | `jmp 0x1564c40` |
| 0x411e30 | CCollision::SortOutCollisionAfterLoad | stub-to-.HOODLUM | `jmp 0x15678d0` |
| 0x4152c0 | CCollision::IsThisVehicleSittingOnMe | stub-to-.HOODLUM | `jmp 0x1566da0` |
| 0x4154a0 | CCollision::CheckPeds | stub-to-.HOODLUM | `jmp 0x1567650` |
| 0x415540 | CCollision::ResetMadeInvisibleObjects | stub-to-.HOODLUM | `jmp 0x156d280` |
| 0x40f070 | CCollisionData::RemoveCollisionVolumes | stub-to-.HOODLUM | `jmp 0x156fae0` |
| 0x40f120 | CCollisionData::Copy | stub-to-.HOODLUM | `jmp 0x1562c90` |
| 0x40f6c0 | CCollisionData::SetLinkPtr | stub-to-.HOODLUM | `jmp 0x1568990` |
| 0x40f6e0 | CCollisionData::GetLinkPtr | stub-to-.HOODLUM | `jmp 0x156d850` |
| 0x41b360 | CTempColModels::Shutdown | stub-to-.HOODLUM | `jmp 0x156cbc0` |
| 0x43c190 | CConversationForPed::Update | stub-to-.HOODLUM | `jmp 0x15668b0` |
| 0x43a7b0 | CConversations::Clear | stub-to-.HOODLUM | `jmp 0x156dff0` |
| 0x43c590 | CConversations::Update | stub-to-.HOODLUM | `jmp 0x156d7c0` |
| 0x43a870 | CConversations::SetUpConversationNode | stub-to-.HOODLUM | `jmp 0x156a7c0` |
| 0x43a960 | CConversations::RemoveConversationForPed | stub-to-.HOODLUM | `jmp 0x156adc0` |
| 0x43b0b0 | CConversations::IsPlayerInPositionForConversation | stub-to-.HOODLUM | `jmp 0x1563250` |
| 0x43aac0 | CConversations::IsConversationGoingOn | stub-to-.HOODLUM | `jmp 0x1570080` |
| 0x43b000 | CConversations::IsConversationAtNode | stub-to-.HOODLUM | `jmp 0x156c500` |
| 0x43a810 | CConversations::AwkwardSay | stub-to-.HOODLUM | `jmp 0x1561cc0` |
| 0x43a840 | CConversations::StartSettingUpConversation | stub-to-.HOODLUM | `jmp 0x1565550` |
| 0x43aae0 | CPedToPlayerConversations::Clear | stub-to-.HOODLUM | `jmp 0x15635e0` |
| 0x43ab10 | CPedToPlayerConversations::EndConversation | stub-to-.HOODLUM | `jmp 0x1566f50` |
| 0x411a80 | CMatrix::operator*= | stub-to-.HOODLUM | `jmp 0x156ff00` |
| 0x8d0aa8 | CCutsceneMgr::FindCutsceneAudioTrackId | outside-game-text(.data@0x8a4000) | `` |
| 0x43d1f0 | CDarkel::FrenzyOnGoing | stub-to-.HOODLUM | `jmp 0x15635c0` |
| 0x43ceb0 | CDarkel::Init | stub-to-.HOODLUM | `jmp 0x15647b0` |
| 0x43d2f0 | CDarkel::ThisPedShouldBeKilledForFrenzy | stub-to-.HOODLUM | `jmp 0x1567170` |
| 0x43d350 | CDarkel::ThisVehicleShouldBeKilledForFrenzy | stub-to-.HOODLUM | `jmp 0x156ced0` |
| 0x43d6a0 | CDarkel::ResetModelsKilledByPlayer | stub-to-.HOODLUM | `jmp 0x1561630` |
| 0x43d6c0 | CDarkel::QueryModelsKilledByPlayer | stub-to-.HOODLUM | `jmp 0x1564c20` |
| 0x43d6e0 | CDarkel::FindTotalPedsKilledByPlayer | stub-to-.HOODLUM | `jmp 0x1569900` |
| 0x46a2d0 | CEntity::GetModellingMatrix | stub-to-.HOODLUM | `jmp 0x1565110` |
| 0x446f90 | CEntity::UpdateRwMatrix | stub-to-.HOODLUM | `jmp 0x1569bf0` |
| 0x4633e0 | CEntity::GetIsStatic | stub-to-.HOODLUM | `jmp 0x15619f0` |
| 0x43aba0 | CPed::PedIsReadyForConversation | stub-to-.HOODLUM | `jmp 0x156cae0` |
| 0x4590f0 | CPed::CreateDeadPedMoney | stub-to-.HOODLUM | `jmp 0x1564140` |
| 0x459180 | CPed::CreateDeadPedPickupCoors- | stub-to-.HOODLUM | `jmp 0x1568940` |
| 0x481090 | CPed::SetStayInSamePlace | stub-to-.HOODLUM | `jmp 0x156d8b0` |
| 0x41be60 | CPlayerPed::GetWantedLevel | stub-to-.HOODLUM | `jmp 0x156d300` |
| 0x420b80 | CPlaceable::SetPosn-xyz | stub-to-.HOODLUM | `jmp 0x1566e30` |
| 0x43e650 | CEntryExit::GetEntryExitToDisplayNameOf | stub-to-.HOODLUM | `jmp 0x1569780` |
| 0x43f880 | CEntryExitManager::Init | stub-to-.HOODLUM | `jmp 0x156a6f0` |
| 0x440b90 | CEntryExitManager::Shutdown | stub-to-.HOODLUM | `jmp 0x1563940` |
| 0x43e410 | CEntryExitManager::AddEntryExitToStack | stub-to-.HOODLUM | `jmp 0x1564090` |
| 0x440c40 | CEntryExitManager::ShutdownForRestart | stub-to-.HOODLUM | `jmp 0x1567550` |
| 0x43fd50 | CEntryExitManager::DeleteOne | stub-to-.HOODLUM | `jmp 0x1560ed0` |
| 0x43f180 | CEntryExitManager::EnableBurglaryHouses | stub-to-.HOODLUM | `jmp 0x156c6d0` |
| 0x43f150 | CEntryExitManager::GetPositionRelativeToOutsideWorld | stub-to-.HOODLUM | `jmp 0x1566880` |
| 0x43f0a0 | CEntryExitManager::PostEntryExitsCreation | stub-to-.HOODLUM | `jmp 0x15631c0` |
| 0x43ed80 | CEntryExitManager::ResetAreaCodeForVisibleObjects | stub-to-.HOODLUM | `jmp 0x15678a0` |
| 0x43ecf0 | CEntryExitManager::SetAreaCodeForVisibleObjects | stub-to-.HOODLUM | `jmp 0x1560930` |
| 0x420e70 | CEventAcquaintancePedHate::Constructor2 | stub-to-.HOODLUM | `jmp 0x1561070` |
| 0x48e1c0 | CEventLeaderEnteredCarAsDriver::Constructor | stub-to-.HOODLUM | `jmp 0x156dc10` |
| 0x43e1c0 | CEventLeaderEntryExit::Constructor | stub-to-.HOODLUM | `jmp 0x1566d50` |
| 0x441560 | CGameLogic::ClearSkip | stub-to-.HOODLUM | `jmp 0x1561270` |
| 0x4428b0 | CGameLogic::DoWeaponStuffAtStartOf2PlayerGame | stub-to-.HOODLUM | `jmp 0x156aa30` |
| 0x441240 | CGameLogic::ForceDeathRestart | stub-to-.HOODLUM | `jmp 0x15672e0` |
| 0x441210 | CGameLogic::InitAtStartOfGame | stub-to-.HOODLUM | `jmp 0x1563910` |
| 0x441390 | CGameLogic::IsCoopGameGoingOn | stub-to-.HOODLUM | `jmp 0x156cf30` |
| 0x442020 | CGameLogic::IsPlayerUse2PlayerControls | stub-to-.HOODLUM | `jmp 0x156ff70` |
| 0x4416c0 | CGameLogic::IsSkipWaitingForScriptToFadeIn | stub-to-.HOODLUM | `jmp 0x1569c30` |
| 0x441d30 | CGameLogic::RestorePedsWeapons | stub-to-.HOODLUM | `jmp 0x1560f90` |
| 0x4423c0 | CGameLogic::SetUpSkip | stub-to-.HOODLUM | `jmp 0x1563e90` |
| 0x4415c0 | CGameLogic::SkipCanBeActivated | stub-to-.HOODLUM | `jmp 0x1564860` |
| 0x443920 | CGangWars::InitAtStartOfGame | stub-to-.HOODLUM | `jmp 0x156b150` |
| 0x443f80 | CGangWars::CanPlayerStartAGangWarHere | stub-to-.HOODLUM | `jmp 0x15699c0` |
| 0x443ff0 | CGangWars::ClearSpecificZonesToTriggerGangWar | stub-to-.HOODLUM | `jmp 0x156dd40` |
| 0x4444b0 | CGangWars::ClearTheStreets | stub-to-.HOODLUM | `jmp 0x1564200` |
| 0x4439c0 | CGangWars::DontCreateCivilians | stub-to-.HOODLUM | `jmp 0x156eff0` |
| 0x443ac0 | CGangWars::GangWarFightingGoingOn | stub-to-.HOODLUM | `jmp 0x1563620` |
| 0x445c30 | CGangWars::ReleasePedsInAttackWave | stub-to-.HOODLUM | `jmp 0x15609c0` |
| 0x445f50 | CGangWars::StrengthenPlayerInfluenceInZone | stub-to-.HOODLUM | `jmp 0x1565090` |
| 0x444530 | CGangWars::TellGangMembersTo | stub-to-.HOODLUM | `jmp 0x1568e00` |
| 0x447d50 | CGarage::OpenThisGarage | stub-to-.HOODLUM | `jmp 0x1567910` |
| 0x447d70 | CGarage::CloseThisGarage | stub-to-.HOODLUM | `jmp 0x156d5c0` |
| 0x4471b0 | CGarages::Shutdown | stub-to-.HOODLUM | `jmp 0x1566e00` |
| 0x449e60 | CGarages::PlayerArrestedOrDied | stub-to-.HOODLUM | `jmp 0x1569150` |
| 0x448b30 | CGarages::AllRespraysCloseOrOpen | stub-to-.HOODLUM | `jmp 0x1563a10` |
| 0x447d00 | CGarages::IsGarageOpen | stub-to-.HOODLUM | `jmp 0x156d2d0` |
| 0x447d30 | CGarages::IsGarageClosed | stub-to-.HOODLUM | `jmp 0x1560f70` |
| 0x4479a0 | CGarages::IsCarSprayable | stub-to-.HOODLUM | `jmp 0x156b1c0` |
| 0x447cd0 | CGarages::ActivateGarage | stub-to-.HOODLUM | `jmp 0x15677f0` |
| 0x447cb0 | CGarages::DeActivateGarage | stub-to-.HOODLUM | `jmp 0x1563e70` |
| 0x447c40 | CGarages::SetTargetCarForMissionGarage | stub-to-.HOODLUM | `jmp 0x1560bd0` |
| 0x447b80 | CGarages::TriggerMessage | stub-to-.HOODLUM | `jmp 0x156cc60` |
| 0x4476d0 | CGarages::ChangeGarageType | stub-to-.HOODLUM | `jmp 0x1561690` |
| 0x447680 | CGarages::GetGarageNumberByName | stub-to-.HOODLUM | `jmp 0x156dcf0` |
| 0x44a210 | CGarages::CountCarsInHideoutGarage | stub-to-.HOODLUM | `jmp 0x1567330` |
| 0x4045b0 | CIplStore::AddIplsNeededAtPosn | stub-to-.HOODLUM | `jmp 0x1567820` |
| 0x405fa0 | CIplStore::Shutdown | stub-to-.HOODLUM | `jmp 0x15610b0` |
| 0x405b60 | CIplStore::RemoveIplSlot | stub-to-.HOODLUM | `jmp 0x156c5f0` |
| 0x405ac0 | CIplStore::AddIplSlot | stub-to-.HOODLUM | `jmp 0x156c470` |
| 0x4058d0 | CIplStore::RemoveIplWhenFarAway | stub-to-.HOODLUM | `jmp 0x156de30` |
| 0x405890 | CIplStore::RemoveIplAndIgnore | stub-to-.HOODLUM | `jmp 0x156a140` |
| 0x405850 | CIplStore::RequestIplAndIgnore | stub-to-.HOODLUM | `jmp 0x1565190` |
| 0x404c90 | CIplStore::IncludeEntity | stub-to-.HOODLUM | `jmp 0x1563730` |
| 0x4047b0 | CIplStore::GetIplEntityIndexArray | stub-to-.HOODLUM | `jmp 0x1569770` |
| 0x404780 | CIplStore::GetNewIplEntityIndexArray | stub-to-.HOODLUM | `jmp 0x15649e0` |
| 0x4045e0 | CIplStore::ClearIplsNeededAtPosn | stub-to-.HOODLUM | `jmp 0x156cff0` |
| 0x4212c0 | CBaseModelInfo::SwaysInWind | stub-to-.HOODLUM | `jmp 0x156c780` |
| 0x41b870 | CNodeRoute::operator delete | stub-to-.HOODLUM | `jmp 0x156e0c0` |
| 0x44cd50 | COnscreenTimer::AddClock | stub-to-.HOODLUM | `jmp 0x156e3e0` |
| 0x44ce60 | COnscreenTimer::ClearClock | stub-to-.HOODLUM | `jmp 0x1565730` |
| 0x44cee0 | COnscreenTimer::SetClockBeepCountdownSecs | stub-to-.HOODLUM | `jmp 0x1562130` |
| 0x44cda0 | COnscreenTimer::AddCounter | stub-to-.HOODLUM | `jmp 0x1561c50` |
| 0x44ce80 | COnscreenTimer::ClearCounter | stub-to-.HOODLUM | `jmp 0x156ad90` |
| 0x44ceb0 | COnscreenTimer::SetCounterFlashWhenFirstDisplayed | stub-to-.HOODLUM | `jmp 0x156ec20` |
| 0x44d3e0 | CPathFind::ThisNodeHasToBeSwitchedOff | stub-to-.HOODLUM | `jmp 0x1561710` |
| 0x44d230 | CPathFind::These2NodesAreAdjacent | stub-to-.HOODLUM | `jmp 0x1569710` |
| 0x44ded0 | CPathFind::AddInteriorLink | stub-to-.HOODLUM | `jmp 0x156ec90` |
| 0x450d70 | CPathFind::MakeRequestForNodesToBeLoaded | stub-to-.HOODLUM | `jmp 0x156a6b0` |
| 0x450950 | CPathFind::Shutdown | stub-to-.HOODLUM | `jmp 0x1562750` |
| 0x4504f0 | CPathFind::CountNeighboursToBeSwitchedOff | stub-to-.HOODLUM | `jmp 0x156a990` |
| 0x450560 | CPathFind::MarkRoadNodeAsDontWander | stub-to-.HOODLUM | `jmp 0x156ecf0` |
| 0x4529f0 | CPathFind::LoadPathFindData-FromStream | stub-to-.HOODLUM | `jmp 0x156f750` |
| 0x452f00 | CPathFind::SwitchPedRoadsOffInArea | stub-to-.HOODLUM | `jmp 0x1565860` |
| 0x452760 | CPathFind::ComputeRoute | stub-to-.HOODLUM | `jmp 0x156b7d0` |
| 0x452160 | CPathFind::SwitchOffNodeAndNeighbours | stub-to-.HOODLUM | `jmp 0x15646a0` |
| 0x452090 | CPathFind::Find2NodesForCarCreation | stub-to-.HOODLUM | `jmp 0x156d150` |
| 0x451350 | CPathFind::FindLinkBetweenNodes | stub-to-.HOODLUM | `jmp 0x156a180` |
| 0x44d480 | CPathFind::TestForPedTrafficLight | stub-to-.HOODLUM | `jmp 0x156a400` |
| 0x44d400 | CPathFind::UnMarkAllRoadNodesAsDontWander | stub-to-.HOODLUM | `jmp 0x1565420` |
| 0x44d3b0 | CPathFind::TidyUpNodeSwitchesAfterMission | stub-to-.HOODLUM | `jmp 0x156de10` |
| 0x44d310 | CPathFind::ThisNodeWillLeadIntoADeadEnd | stub-to-.HOODLUM | `jmp 0x1569f30` |
| 0x44d0f0 | CPathFind::UnLoadPathFindData | stub-to-.HOODLUM | `jmp 0x1563fb0` |
| 0x44d080 | CPathFind::Init | stub-to-.HOODLUM | `jmp 0x1560dc0` |
| 0x44d790 | CPathFind::TestCrossesRoad | stub-to-.HOODLUM | `jmp 0x1565af0` |
| 0x44e4e0 | CPathFind::ReInit | stub-to-.HOODLUM | `jmp 0x156a600` |
| 0x44e1a0 | CPathFind::RemoveInterior | stub-to-.HOODLUM | `jmp 0x1564320` |
| 0x44e000 | CPathFind::AddDynamicLinkBetween2Nodes_For1Node | stub-to-.HOODLUM | `jmp 0x1560c20` |
| 0x44de80 | CPathFind::StartNewInterior | stub-to-.HOODLUM | `jmp 0x156ad40` |
| 0x44de00 | CPathFind::LoadSceneForPathNodes | stub-to-.HOODLUM | `jmp 0x1565690` |
| 0x44dd00 | CPathFind::ReleaseRequestedNodes | stub-to-.HOODLUM | `jmp 0x1561b10` |
| 0x44dcd0 | CPathFind::SetPathsNeededAtPosition | stub-to-.HOODLUM | `jmp 0x156de80` |
| 0x4893e0 | CPedIntelligence::GetPedEntities | stub-to-.HOODLUM | `jmp 0x156aa00` |
| 0x454be0 | CPickup::ExtractAmmoFromPickup | stub-to-.HOODLUM | `jmp 0x156db40` |
| 0x455540 | CPickup::FindStringForTextIndex | stub-to-.HOODLUM | `jmp 0x1565750` |
| 0x455500 | CPickup::FindTextIndexForString | stub-to-.HOODLUM | `jmp 0x156ec50` |
| 0x4588b0 | CPickup::ProcessGunShot | stub-to-.HOODLUM | `jmp 0x1562550` |
| 0x454a70 | CPickups::Init | stub-to-.HOODLUM | `jmp 0x1564650` |
| 0x455240 | CPickups::AddToCollectedPickupsArray | stub-to-.HOODLUM | `jmp 0x1561580` |
| 0x4551c0 | CPickups::FindPickUpForThisObject | stub-to-.HOODLUM | `jmp 0x156d870` |
| 0x457380 | CPickups::GenerateNewOne_WeaponType | stub-to-.HOODLUM | `jmp 0x156df10` |
| 0x4552a0 | CPickups::GetActualPickupIndex | stub-to-.HOODLUM | `jmp 0x1564bb0` |
| 0x454b40 | CPickups::IsPickUpPickedUp | stub-to-.HOODLUM | `jmp 0x156da60` |
| 0x454ac0 | CPickups::ModelForWeapon | stub-to-.HOODLUM | `jmp 0x1568bd0` |
| 0x4554c0 | CPickups::PlayerCanPickUpThisWeaponTypeAtThisMoment | stub-to-.HOODLUM | `jmp 0x156a950` |
| 0x455680 | CPickups::UpdateMoneyPerDay | stub-to-.HOODLUM | `jmp 0x156b360` |
| 0x41b310 | CCollisionPlugin::PluginAttach | stub-to-.HOODLUM | `jmp 0x15671d0` |
| 0x405900 | CIplDefPool::Constructor | stub-to-.HOODLUM | `jmp 0x1561b20` |
| 0x404870 | CObjectPool::GetAt | stub-to-.HOODLUM | `jmp 0x156de60` |
| 0x406f50 | CPopulation::DoesCarGroupHaveModelId | stub-to-.HOODLUM | `jmp 0x15642e0` |
| 0x45b150 | CReplay::DisableReplays | stub-to-.HOODLUM | `jmp 0x1564640` |
| 0x45b160 | CReplay::EnableReplays | stub-to-.HOODLUM | `jmp 0x1568bc0` |
| 0x45ebb0 | CReplay::RecordVehicleDeleted | stub-to-.HOODLUM | `jmp 0x156dab0` |
| 0x45ec20 | CReplay::RecordPedDeleted | stub-to-.HOODLUM | `jmp 0x1561870` |
| 0x45c440 | CReplay::ShouldStandardCameraBeProcessed | stub-to-.HOODLUM | `jmp 0x156e610` |
| 0x45f180 | CReplay::StoreStuffInMem | stub-to-.HOODLUM | `jmp 0x15689d0` |

## Procedures by region and gta-reversed state

Procedure rows: 26267 (20823 Ghidra, 5444 added: callee 416, cat7-call 63, data-code-immediate 595, data-crt-init 1666, data-fnptr-table 58, data-fnptr-table-unreferenced 17, data-fnptr-variable 1, data-rw-device 4, data-rw-node-def 8, data-script-range-fns 24, data-struct-array 4, data-vtable 343, hook 1894, vtable 351) (+214 imports).

Each function is counted once. 492 `.HOODLUM` procedures are bodies of game-.text `jmp` stubs: they are listed as `stub-body` and counted at the stub (CAT-9). The catalogue proper is game-text + rw-cseg: **22162** functions, the same total as `subsystems.md` and the rows of `procedures.csv`.

| Region | gr-reversed | gr-unreversed | gr-called-only | unknown | functions | stub-body (counted at the stub) |
|---|---|---|---|---|---|---|
| game-text | 7900 | 730 | 944 | 12580 | 22154 | 0 |
| hoodlum | 0 | 0 | 0 | 0 | 0 | 492 |
| other | 0 | 0 | 0 | 1615 | 1615 | 0 |
| protection-text | 0 | 0 | 0 | 1998 | 1998 | 0 |
| rw-cseg | 0 | 0 | 0 | 8 | 8 | 0 |
| **total** | 7900 | 730 | 944 | 16201 | 25775 | 492 |

Imports by DLL: KERNEL32.DLL 137, USER32.DLL 44, VORBISFILE.DLL 7, WS2_32.DLL 7, ADVAPI32.DLL 6, WINMM.DLL 4, OLE32.DLL 3, EAX.DLL 1, GDI32.DLL 1, D3D9.DLL 1, DINPUT8.DLL 1, DSOUND.DLL 1, DDRAW.DLL 1

## Reconciliation (CAT-9, generated by subsystems.py)

- `procedures.csv` rows = `subsystems.md` total = **22162** functions (game .text + _rwcseg). Each function is one row; a `.HOODLUM` body is part of its stub's row.
- Distinct gta-reversed hook addresses: 8632. In the catalogue: 8630 (gr-reversed 7900, gr-unreversed 730; 379 of them are `.HOODLUM` stubs, counted once at the stub). Not in the catalogue: 2 (0x8d0aa8 CCutsceneMgr::FindCutsceneAudioTrackId outside-game-text(.data@0x8a4000), 0xdeadbeef CPedAttractor::GetNoOfRegisteredPeds outside-image).
- The registration counts at the top (with duplicates, first registration of an address decides its reversed flag here) are source counts, not function counts.
