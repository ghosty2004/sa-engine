//! Phase B registrations (`#[native]`) of existing ports whose entries cannot be hooked in phase A
//! (STD-6: their docs say "not hookable"; collected by address).

/// Every `#[native]` STD-6 added to existing ports of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::gen_bodies::NATIVE_G_CAESCRIPT_AUDIO_ENTITY_ATTACH_MISSION_AUDIO_TO_PHYSICAL_4EC100,
    crate::gen_dtors::NATIVE_G_DTOR_84C1A0_8562E0,
    crate::gen_dtors::NATIVE_G_DTOR_84C4F0_856310,
    crate::gen_dtors::NATIVE_G_DTOR_AESTREAM_TRANSFORMER_856320,
    crate::gen_dtors::NATIVE_G_DTOR_84CC30_856350,
    crate::gen_dtors::NATIVE_G_DTOR_84CD60_856370,
    crate::gen_leaves::NATIVE_G_CAEPED_AUDIO_ENTITY_PLAY_WIND_RUSH_4E1170,
    crate::gen_leaves::NATIVE_G_CAEPEDLESS_SPEECH_AUDIO_ENTITY_ADD_SCRIPT_SAY_EVENT_4E6080,
    crate::gen_leaves::NATIVE_G_CAEPEDLESS_SPEECH_AUDIO_ENTITY_IS_PED_FEMALE_FOR_AUDIO_4E60C0,
    crate::gen_leaves::NATIVE_G_CAERADIO_TRACK_MANAGER_HAS_RADIO_RETUNE_JUST_STARTED_4E8370,
    crate::gen_leaves::NATIVE_G_CAERADIO_TRACK_MANAGER_GET_RADIO_STATION_LISTEN_TIMES_4E83E0,
    crate::gen_leaves::NATIVE_G_CAERADIO_TRACK_MANAGER_PLAY_RADIO_ANNOUNCEMENT_4E8400,
    crate::gen_leaves::NATIVE_G_CAEVEHICLE_AUDIO_ENTITY_STATIC_SERVICE_4F4EC0,
];
