//! `ModelIndices`: the `MI_*` model-index globals (16-bit cells, 4 bytes apart, at 0x8CD4F4–0x8CD780),
//! `InitModelIndices` and `MatchAllModelStrings`.
//!
//! Ported from the 1.0 US bytes (gta-reversed `ModelIndices.cpp` is a reading aid). The tables below
//! are read from the two functions' instructions (`mov word [cell], ax` / `push cell; push name; call
//! CModelInfo::GetModelInfoUInt16`). Kept from the original: 44 cells are matched but never reset,
//! one (0x8CD5B8) is reset but never matched, and a name that is not found leaves 0 (not -1) in its
//! cell (`GetModelInfoUInt16` writes its zero-initialised index).
//!
//! The cells are declared as two `u16` arrays (0x8CD730–0x8CD73F is not a model index). The high
//! halves of the 4-byte slots are not written by these functions.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::base::MiPtr;

sa_core::globals! {
    list GLOBALS;
    /// `MI_*` cells 0x8CD4F4..0x8CD730 (low halves 0xFFFF in the image)
    pub static MODEL_INDICES_A @ 0x8CD4F4: [u16; 0x11E] = [0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0];
    /// `MI_*` cells 0x8CD740..0x8CD782
    pub static MODEL_INDICES_B @ 0x8CD740: [u16; 0x21] = [0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF];
}

orig! {
    /// `CModelInfo::GetModelInfoUInt16` (0x4C59F0)
    pub fn get_model_info_uint16(name: GPtr<u8>, out: GPtr<u16>) -> MiPtr = cdecl 0x4C59F0 => super::model_info::get_model_info_uint16;
}

/// The cells `InitModelIndices` sets to 0xFFFF (16-bit stores), in the original's order.
pub const RESET: [u32; 116] = [
    0x8CD4F4, 0x8CD4F8, 0x8CD4FC, 0x8CD500, 0x8CD504, 0x8CD518, 0x8CD51C, 0x8CD520,
    0x8CD524, 0x8CD528, 0x8CD52C, 0x8CD530, 0x8CD534, 0x8CD538, 0x8CD53C, 0x8CD540,
    0x8CD544, 0x8CD548, 0x8CD54C, 0x8CD550, 0x8CD554, 0x8CD558, 0x8CD55C, 0x8CD560,
    0x8CD564, 0x8CD568, 0x8CD56C, 0x8CD570, 0x8CD574, 0x8CD578, 0x8CD57C, 0x8CD580,
    0x8CD584, 0x8CD588, 0x8CD58C, 0x8CD590, 0x8CD594, 0x8CD598, 0x8CD59C, 0x8CD5A0,
    0x8CD5A4, 0x8CD5A8, 0x8CD5AC, 0x8CD5B0, 0x8CD5B4, 0x8CD5B8, 0x8CD5BC, 0x8CD5C0,
    0x8CD5C4, 0x8CD5C8, 0x8CD5E8, 0x8CD5EC, 0x8CD5CC, 0x8CD5D0, 0x8CD5D4, 0x8CD5D8,
    0x8CD5DC, 0x8CD5E0, 0x8CD5E4, 0x8CD5F0, 0x8CD5F4, 0x8CD5F8, 0x8CD5FC, 0x8CD74C,
    0x8CD750, 0x8CD754, 0x8CD610, 0x8CD614, 0x8CD618, 0x8CD61C, 0x8CD620, 0x8CD624,
    0x8CD628, 0x8CD62C, 0x8CD630, 0x8CD634, 0x8CD638, 0x8CD63C, 0x8CD640, 0x8CD644,
    0x8CD648, 0x8CD64C, 0x8CD650, 0x8CD654, 0x8CD658, 0x8CD65C, 0x8CD660, 0x8CD664,
    0x8CD668, 0x8CD66C, 0x8CD670, 0x8CD674, 0x8CD678, 0x8CD67C, 0x8CD680, 0x8CD684,
    0x8CD688, 0x8CD68C, 0x8CD690, 0x8CD694, 0x8CD698, 0x8CD69C, 0x8CD6A0, 0x8CD6A4,
    0x8CD6A8, 0x8CD508, 0x8CD50C, 0x8CD510, 0x8CD514, 0x8CD748, 0x8CD76C, 0x8CD770,
    0x8CD774, 0x8CD778, 0x8CD77C, 0x8CD780,
];
/// `(name, cell)` pairs `MatchAllModelStrings` passes to `GetModelInfoUInt16`, in order.
pub const MATCH: [(u32, u32); 160] = [
    (0x869620, 0x8CD594), // "fire_hydrant"
    (0x869614, 0x8CD590), // "phonesign"
    (0x869604, 0x8CD58C), // "noparkingsign1"
    (0x8695F8, 0x8CD588), // "bussign1"
    (0x8695E4, 0x8CD57C), // "DYN_ROADBARRIER_6"
    (0x8695D8, 0x8CD580), // "la_fuckcar1"
    (0x8695CC, 0x8CD584), // "la_fuckcar2"
    (0x8695C4, 0x8CD578), // "dump1"
    (0x8695B8, 0x8CD574), // "trafficcone"
    (0x8695AC, 0x8CD570), // "newsstand1"
    (0x8695A0, 0x8CD56C), // "postbox1"
    (0x869598, 0x8CD568), // "bin1"
    (0x86958C, 0x8CD564), // "wastebin"
    (0x869580, 0x8CD560), // "phonebooth1"
    (0x869570, 0x8CD548), // "parkingmeter"
    (0x869560, 0x8CD54C), // "parkingmeterg"
    (0x869554, 0x8CD550), // "mall_fans"
    (0x869540, 0x8CD554), // "htl_fan_rotate_nt"
    (0x86952C, 0x8CD558), // "htl_fan_rotate_dy"
    (0x869520, 0x8CD55C), // "hotroomfan"
    (0x869510, 0x8CD4F4), // "trafficlight1"
    (0x869504, 0x8CD4F8), // "MTraffic4"
    (0x8694F8, 0x8CD4FC), // "MTraffic1"
    (0x8694E4, 0x8CD500), // "vgsstriptlights1"
    (0x8694D8, 0x8CD504), // "MTraffic2"
    (0x8694CC, 0x8CD518), // "lamppost1"
    (0x8694C0, 0x8CD51C), // "lamppost2"
    (0x8694B4, 0x8CD520), // "lamppost3"
    (0x8694A0, 0x8CD524), // "doublestreetlght1"
    (0x869494, 0x8CD528), // "Streetlamp1"
    (0x869488, 0x8CD52C), // "Streetlamp2"
    (0x869478, 0x8CD530), // "doc_crane_cab0"
    (0x869468, 0x8CD534), // "doc_crane_cab01"
    (0x869458, 0x8CD538), // "doc_crane_cab02"
    (0x869448, 0x8CD53C), // "doc_crane_cab03"
    (0x869438, 0x8CD540), // "boatcranelg0"
    (0x86942C, 0x8CD544), // "LODnetopa0"
    (0x869420, 0x8CD598), // "package1"
    (0x869418, 0x8CD59C), // "Money"
    (0x869410, 0x8CD5A0), // "barrel1"
    (0x869408, 0x8CD5A4), // "barrel2"
    (0x8693FC, 0x8CD5A8), // "DYN_TV_2"
    (0x8693F0, 0x8CD5AC), // "briefcase"
    (0x8693E4, 0x8CD5B0), // "wglasssmash"
    (0x8693D4, 0x8CD5B4), // "glassfx_composh"
    (0x8693C8, 0x8CD5BC), // "adrenaline"
    (0x8693BC, 0x8CD5C0), // "bodyarmour"
    (0x868A3C, 0x8CD5C4), // "info"
    (0x85A4F0, 0x8CD5C8), // "health"
    (0x8693B4, 0x8CD5CC), // "bonus"
    (0x8693AC, 0x8CD5D0), // "bribe"
    (0x8693A0, 0x8CD5D4), // "killfrenzy"
    (0x869390, 0x8CD5D8), // "camerapickup"
    (0x869384, 0x8CD5DC), // "gun_para"
    (0x869378, 0x8CD74C), // "para_pack"
    (0x86936C, 0x8CD5E0), // "bigdollar"
    (0x869360, 0x8CD5E4), // "pickupsave"
    (0x869350, 0x8CD5E8), // "property_locked"
    (0x869340, 0x8CD5EC), // "property_fsale"
    (0x869334, 0x8CD5F0), // "clothesp"
    (0x869324, 0x8CD5F4), // "killfrenzy2plyr"
    (0x86931C, 0x8CD5F8), // "2player"
    (0x86930C, 0x8CD5FC), // "bollardlight"
    (0x8692FC, 0x8CD600), // "bar_barrier10"
    (0x8692EC, 0x8CD604), // "bar_barrier12"
    (0x8692E4, 0x8CD608), // "bouy"
    (0x8692D8, 0x8CD60C), // "parktable1"
    (0x8692CC, 0x8CD750), // "CJ_OYSTER"
    (0x8692BC, 0x8CD754), // "cj_horse_Shoe"
    (0x8692B0, 0x8CD758), // "wheel_or1"
    (0x8692A8, 0x8CD760), // "nto_b_s"
    (0x8692A0, 0x8CD764), // "nto_b_l"
    (0x869294, 0x8CD768), // "nto_b_tw"
    (0x8694CC, 0x8CD610), // "lamppost1"
    (0x869288, 0x8CD614), // "mlamppost"
    (0x869274, 0x8CD618), // "roadworkbarrier1"
    (0x869264, 0x8CD61C), // "littleha_police"
    (0x869254, 0x8CD620), // "telgrphpole02"
    (0x869510, 0x8CD624), // "trafficlight1"
    (0x869248, 0x8CD628), // "parkbench1"
    (0x869418, 0x8CD59C), // "Money"
    (0x869238, 0x8CD62C), // "od_lightbeam"
    (0x869228, 0x8CD630), // "ap_radar1_01"
    (0x869220, 0x8CD634), // "rcbomb"
    (0x869214, 0x8CD638), // "beachball"
    (0x869208, 0x8CD63C), // "sandcastle1"
    (0x8691FC, 0x8CD640), // "sandcastle2"
    (0x8691F0, 0x8CD644), // "jellyfish"
    (0x8691E4, 0x8CD648), // "jellyfish01"
    (0x8691D8, 0x8CD64C), // "fish1single"
    (0x8691D0, 0x8CD650), // "fish1s"
    (0x8691C4, 0x8CD654), // "fish2single"
    (0x8691BC, 0x8CD658), // "fish2s"
    (0x8691B0, 0x8CD65C), // "fish3single"
    (0x8691A8, 0x8CD660), // "fish3s"
    (0x8691A0, 0x8CD664), // "turtle"
    (0x869198, 0x8CD668), // "dolphin"
    (0x869190, 0x8CD66C), // "shark"
    (0x869184, 0x8CD670), // "submarine"
    (0x869178, 0x8CD674), // "Esc_step"
    (0x86916C, 0x8CD678), // "Esc_step8"
    (0x86915C, 0x8CD67C), // "lounge_wood_up"
    (0x86914C, 0x8CD680), // "lounge_towel_up"
    (0x86913C, 0x8CD684), // "lounge_wood_dn"
    (0x869134, 0x8CD688), // "lotion"
    (0x869124, 0x8CD68C), // "beachtowel01"
    (0x869114, 0x8CD690), // "beachtowel02"
    (0x869104, 0x8CD694), // "beachtowel03"
    (0x8690F4, 0x8CD698), // "beachtowel04"
    (0x8690E8, 0x8CD69C), // "blimp_night"
    (0x8690DC, 0x8CD6A0), // "blimp_day"
    (0x8690CC, 0x8CD6A4), // "yt_main_body"
    (0x8690BC, 0x8CD6A8), // "yt_main_body2"
    (0x8690B0, 0x8CD6AC), // "mil_samsite"
    (0x8690A0, 0x8CD6B0), // "samsite_SFXRF"
    (0x869094, 0x8CD6B4), // "traincross2"
    (0x869088, 0x8CD6B8), // "traincross1"
    (0x869078, 0x8CD6BC), // "MagnoCrane_03"
    (0x869068, 0x8CD6C0), // "TwrCrane_M_02"
    (0x869058, 0x8CD6C4), // "quarry_cranearm"
    (0x869044, 0x8CD6C8), // "kmb_container_red"
    (0x869030, 0x8CD6CC), // "kmb_container_yel"
    (0x86901C, 0x8CD6D0), // "kmb_container_blue"
    (0x869014, 0x8CD6D4), // "barrel4"
    (0x869000, 0x8CD6D8), // "kmb_container_open"
    (0x868FF4, 0x8CD6DC), // "blockpallet"
    (0x868FE4, 0x8CD6E0), // "magnocrane5_SFS"
    (0x868FD8, 0x8CD6E4), // "kmb_deadarm"
    (0x868FCC, 0x8CD6E8), // "kmb_deadleg"
    (0x868FBC, 0x8CD6EC), // "kmb_deadhead"
    (0x868FAC, 0x8CD6F0), // "kmb_deadtorso"
    (0x868FA0, 0x8CD6F4), // "grasshouse"
    (0x868F94, 0x8CD6F8), // "grassplant"
    (0x868F84, 0x8CD700), // "INDUS_MAGNET"
    (0x868F70, 0x8CD6FC), // "CJ_PLAYER_HARNESS"
    (0x868F5C, 0x8CD704), // "dyn_quarryrock01"
    (0x868F48, 0x8CD708), // "dyn_quarryrock02"
    (0x868F34, 0x8CD70C), // "dyn_quarryrock03"
    (0x868F28, 0x8CD710), // "KMB_ATM3"
    (0x868F18, 0x8CD714), // "dead_tied_cop"
    (0x868F08, 0x8CD718), // "bonyrd_windsock"
    (0x868EF4, 0x8CD71C), // "DYN_WREKING_BALL"
    (0x868EEC, 0x8CD720), // "bomb"
    (0x868EE0, 0x8CD740), // "mini_magnet"
    (0x868ED4, 0x8CD744), // "ab_carcass"
    (0x868EC8, 0x8CD724), // "wong_dish"
    (0x868EB8, 0x8CD728), // "CJ_BEAR_BOTTLE"
    (0x868EAC, 0x8CD72C), // "CJ_CIGGY"
    (0x868E98, 0x8CD508), // "CJ_TRAFFIC_LIGHT3"
    (0x868E84, 0x8CD50C), // "CJ_TRAFFIC_LIGHT4"
    (0x868E70, 0x8CD510), // "CJ_TRAFFIC_LIGHT5"
    (0x868E5C, 0x8CD514), // "GAY_TRAFFIC_LIGHT"
    (0x868E4C, 0x8CD748), // "imy_shash_wall"
    (0x868E44, 0x8CD75C), // "flare"
    (0x85BB28, 0x8CD76C), // "hydralics"
    (0x85BB20, 0x8CD770), // "stereo"
    (0x868E38, 0x8CD774), // "BBALL_COL"
    (0x868E28, 0x8CD778), // "k_poolballcue"
    (0x868E1C, 0x8CD77C), // "punchbagnu"
    (0x868E10, 0x8CD780), // "imy_bbox"
];

/// The cell at 1.0 US address `addr` (inside one of the two arrays).
pub fn cell(addr: u32) -> GPtr<u16> {
    if addr < 0x8CD740 {
        MODEL_INDICES_A.ptr().cast::<u8>().byte_add((addr - 0x8CD4F4) as isize).cast()
    } else {
        MODEL_INDICES_B.ptr().cast::<u8>().byte_add((addr - 0x8CD740) as isize).cast()
    }
}

/// `ModelIndices::InitModelIndices` (0x5BCA10): 0xFFFF into every cell of [`RESET`].
#[hook(addr = 0x5BCA10, conv = cdecl, name = "ModelIndices::InitModelIndices", expect = "B8 FF FF 00 00")]
pub fn init_model_indices() {
    for a in RESET {
        cell(a).write(0xFFFF);
    }
}

/// `ModelIndices::MatchAllModelStrings` (0x5B57C0): `GetModelInfoUInt16(name, &cell)` for every pair
/// of [`MATCH`].
#[hook(addr = 0x5B57C0, conv = cdecl, name = "ModelIndices::MatchAllModelStrings", expect = "68 94 D5 8C 00")]
pub fn match_all_model_strings() {
    for (name, c) in MATCH {
        get_model_info_uint16(GPtr::from_addr(name as usize), cell(c));
    }
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[HOOK_INIT_MODEL_INDICES, HOOK_MATCH_ALL_MODEL_STRINGS];
