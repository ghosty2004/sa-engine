"""The crate layers of ADR-003 §3.4, lowest first. A crate may depend only on crates of a lower layer
(`tools/verify/crate_layers.py` checks it in the writer gate; `tools/gen/bindings.py` places bindings with it).

The order is the one the code has (measured 2026-11-14): `sa-pop` sits below `sa-veh` / `sa-cam` / `sa-render`
(they call its car-control and population code), `sa-anim` / `sa-input` beside `sa-stream`.
"""

LAYERS = [
    # leaves: no engine dependency
    ['sa-macros', 'sa-math', 'sa-platform', 'sa-oracle-format', 'sa-samp-api', 'sa-net',
     'sa-launcher', 'sa-udp-proxy', 'sa-build'],
    ['sa-core'],
    ['sa-rwcore', 'sa-stream', 'sa-input', 'sa-anim'],
    ['sa-world'],
    ['sa-pop'],
    ['sa-ped', 'sa-veh', 'sa-cam', 'sa-ai', 'sa-weapon', 'sa-render'],
    ['sa-script', 'sa-hud', 'sa-audio'],
    # the SA:MP glue, the registry of every port
    ['sa-samp', 'sa-ports'],
    ['sa-rw-wgpu', 'sa-hook'],
    ['sa-game', 'sa-oracle'],
]
RANK = {c: i for i, layer in enumerate(LAYERS) for c in layer}


def rank(crate):
    return RANK.get(crate)


def may_depend(crate, dep):
    """True when `crate` may depend on `dep` (a lower layer)."""
    a, b = RANK.get(crate), RANK.get(dep)
    return a is not None and b is not None and b < a
