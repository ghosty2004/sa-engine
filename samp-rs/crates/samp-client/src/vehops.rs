//! Vehicle steps on the game adapter that the DLL client and the hosted client share.

use sa_samp_api::streaming::Streaming;
use sa_samp_api::vehicles::Vehicles;
use sa_samp_api::{GameResult, ModelId, VehicleRef};

/// Polls of a component load after its request, `wait` apart.
pub const COMPONENT_LOAD_POLLS: u32 = 20;

/// samp's add-component (RPC 164's components and SCM event 2): a component that is not loaded is requested
/// (`REQUEST_MODEL`, `LOAD_ALL_MODELS_NOW`, `REQUEST_VEHICLE_MOD`) and polled up to [`COMPONENT_LOAD_POLLS`] times,
/// `wait` apart; a loaded one is added (`ADD_VEHICLE_MOD`), one that did not load is dropped.
pub fn add_component<G: Streaming + Vehicles + ?Sized>(
    game: &mut G,
    v: VehicleRef,
    component: ModelId,
    wait: &dyn Fn(),
) -> GameResult<()> {
    if !game.component_loaded(component) {
        let _ = game.request_model(component);
        let _ = game.load_all_now();
        let _ = game.request_component(component);
        for _ in 0..COMPONENT_LOAD_POLLS {
            if game.component_loaded(component) {
                break;
            }
            wait();
        }
    }
    if game.component_loaded(component) { game.add_component(v, component) } else { Ok(()) }
}
