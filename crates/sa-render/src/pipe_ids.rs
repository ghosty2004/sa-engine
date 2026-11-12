//! Pipeline plugin id accessors and the building / car pipeline attachment checks (STD-6, batch
//! `std6-render-1`). Ported from the 1.0 US bytes (0x72FC40, 0x72FC50, 0x5D6E90, 0x5D7F40,
//! 0x5D5B80).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd};

orig! {
    /// `PipelinePlugin::GetPipelineID(atomic)` (0x72FC40)
    fn get_pipeline_id_call(atomic: GPtr<u8>) -> u32 = cdecl 0x72FC40 => get_pipeline_id;
    /// `CCustomBuildingDNPipeline::GetExtraVertColourPtr(geometry)` (0x5D6E90)
    fn get_extra_vert_colour_ptr_call(geom: GPtr<u8>) -> u32 = cdecl 0x5D6E90 => get_extra_vert_colour_ptr;
}

/// `PipelinePlugin` offset in RpAtomic (`[0x8D6080]`) and the DN pipeline's extra vertex colour
/// offset in RpGeometry (`[0x8D12BC]`).
const PIPELINE_OFFSET: usize = 0x8D6080;
const EXTRA_VERT_COLOUR_OFFSET: usize = 0x8D12BC;

fn field(obj: GPtr<u8>, off_global: usize) -> GPtr<u32> {
    obj.byte_add(g::<i32>(off_global).read() as isize).cast()
}

/// `PipelinePlugin::GetPipelineID(atomic)` (0x72FC40).
#[hook(addr = 0x72FC40, conv = cdecl, name = "PipelinePlugin::GetPipelineID", expect = "8B 44 24 04 8B 0D 80 60 8D 00",
       route = "L1-pure", hazards = "none")]
pub fn get_pipeline_id(atomic: GPtr<u8>) -> u32 {
    field(atomic, PIPELINE_OFFSET).read()
}

/// `PipelinePlugin::SetPipelineID(atomic, id)` (0x72FC50).
#[hook(addr = 0x72FC50, conv = cdecl, name = "PipelinePlugin::SetPipelineID", expect = "8B 44 24 08 8B 4C 24 04 8B 15 80 60 8D 00",
       route = "L1-pure", hazards = "none")]
pub fn set_pipeline_id(atomic: GPtr<u8>, id: u32) {
    field(atomic, PIPELINE_OFFSET).write(id);
}

/// `CCustomBuildingDNPipeline::GetExtraVertColourPtr(geometry)` (0x5D6E90): the night colours.
#[hook(addr = 0x5D6E90, conv = cdecl, name = "CCustomBuildingDNPipeline::GetExtraVertColourPtr", expect = "8B 44 24 04 8B 0D BC 12 8D 00 8B 04",
       route = "L1-pure", hazards = "none")]
pub fn get_extra_vert_colour_ptr(geom: GPtr<u8>) -> u32 {
    field(geom, EXTRA_VERT_COLOUR_OFFSET).read()
}

/// `CCustomBuildingRenderer::IsCBPCPipelineAttached(atomic)` (0x5D7F40; the second compare lives in
/// the protection chunk 0x401B27): 1 for pipeline id 0x53F2009C or 0x53F20098, else 1 when the
/// geometry has night colours and a non-zero +0x30 word, else 0.
#[hook(addr = 0x5D7F40, conv = cdecl, name = "CCustomBuildingRenderer::IsCBPCPipelineAttached", expect = "56 8B 74 24 08 56",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn is_cbpc_pipeline_attached(atomic: GPtr<u8>) -> u32 {
    let id = get_pipeline_id_call(atomic);
    let geom = GPtr::<u8>::from_addr(rd::<u32>(atomic, 0x18) as usize);
    if id == 0x53F2_009C || id == 0x53F2_0098 {
        return 1;
    }
    (get_extra_vert_colour_ptr_call(geom) != 0 && rd::<u32>(geom, 0x30) != 0) as u32
}

/// `CCarFXRenderer::IsCCPCPipelineAttached(atomic)` (0x5D5B80; the compare lives in the protection
/// chunk 0x4067FD): pipeline id == 0x53F2009A.
#[hook(addr = 0x5D5B80, conv = cdecl, name = "CCarFXRenderer::IsCCPCPipelineAttached", expect = "8B 44 24 04 50 E8",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn is_ccpc_pipeline_attached(atomic: GPtr<u8>) -> u32 {
    (get_pipeline_id_call(atomic) == 0x53F2_009A) as u32
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_PIPELINE_ID,
    HOOK_SET_PIPELINE_ID,
    HOOK_GET_EXTRA_VERT_COLOUR_PTR,
    HOOK_IS_CBPC_PIPELINE_ATTACHED,
    HOOK_IS_CCPC_PIPELINE_ATTACHED,
];
