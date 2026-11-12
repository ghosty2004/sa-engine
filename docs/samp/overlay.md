# SAMP-UI: the SA:MP overlay drawn by the engine

Status: first working slice (2026-11-12). Chat window and input line, all six dialog styles, nametags with
health / armour bars, 3D text labels, textdraws (fonts 0–3), game text, the kill list, the scoreboard and the
cursor are drawn by native sa-game through ported engine code, driven by window events from the platform layer.
Evidence: `sa-game --samp-ui-demo` (offscreen PNGs, §6).

Inputs: `docs/samp/game-adapter.md` §4.10 (overlay surface), `docs/samp/native-plan.md` §3 (dispatch points),
samp-rs's `samp-ui` crate (`~/samp-rs-adapter/crates/samp-ui`, read only: its specification of samp.dll's UI
layout, colours, colour codes and wrapping) and `samp-client/src/ui/render/*.rs` (how samp-rs draws today).

## 1. Why the engine draws the UI

samp-rs draws its UI with Direct3D 9 + D3DX through a device proxy that replaces `_RwD3DDevice` (0xC97C28).
Native sa-game (Linux, later wasm32) has no D3D9 device and must not grow a second GPU path. So the UI is
split in two:

| part | where | what |
|---|---|---|
| state, layout, input | `crates/sa-samp-ui` (new, renderer-independent, no engine dependency) | chat (100 entries, page of 10), input line + history, dialogs, nametags, labels, textdraws, kill list, scoreboard, cursor; produces a **draw list** (rectangles, UI-font text runs, game-font text, triangles) |
| engine renderer | `crates/sa-samp/src/overlay/` (new module of the native backend) | draws the list at `FramePhase::After2d` (textdraws) and `FramePhase::Present` (the rest) through `orig!` calls to the ports; owns the glyph atlas raster and the sprite cache |
| dispatch point | `crates/sa-core/src/mvpb_app.rs` (one line) | `FramePhase::Present` in the `Game::Idle` port, after `CFont::DrawFonts` / the fade / the after-fade HUD and before `RwCameraEndUpdate` (samp's D3D9 `Present` hook draws there) |
| API | `crates/sa-samp-api/src/ui.rs` (new, additive) | `SampUi` trait + spec types + `UiEvent`; `GameAdapter::ui()` (default `None`) |
| host | `crates/sa-game/src/samp_input.rs`, `samp_ui_demo.rs` (new) | platform input → window events → UI; the demo |

samp-rs keeps its D3D renderer on the original exe (`GameAdapter::ui()` is `None` there). On the native
backend it feeds the RPC state into `game.ui()` instead of its own `samp-ui` model and drains the events.

## 2. Rendering (engine paths only)

Two passes, where samp draws, each after the client's own frame callback; nothing while the pause menu is active
(samp draws no UI then):

* `After2d` (sa-core's dispatch point after `Render2dStuff`): queued game text goes to `CMessages`, and the
  textdraws are drawn — sprites immediately, text printed into `CFont`'s buffer, which `Game::Idle` flushes with
  the HUD's own text (samp's `frame_after_2d`).
* `Present` (new one-line dispatch point at the end of the `Game::Idle` port, still inside the camera's
  `BeginUpdate`/`EndUpdate`): nametags, labels, bubbles, kill list, chat, input, scoreboard, dialogs, cursor —
  over the game text, the fade and the after-fade HUD, as samp's D3D9 `Present` pass. `FrontendIdle` (the front
  end) has no `Present` dispatch yet; samp draws no UI there anyway.

| draw-list item | engine path (all `orig!` → the Rust port through the address registry) |
|---|---|
| filled rectangle (dialog background, bars, boxes, list selection) | `CSprite2d::DrawRect(CRect, CRGBA)` 0x727B60 |
| UI-font text | RW Im2D: `RwIm2DRenderPrimitive` 0x734E90, `rwPRIMTYPETRILIST`, one quad per glyph, texture = one `RwRaster` (`RwRasterCreate` 0x7FB230 `rwRASTERTYPETEXTURE \| FORMAT8888`, `RwRasterLock` 0x7FB2D0 / `Unlock` 0x7FAEC0) holding the glyph atlas; render states through `RwRenderStateSet` (`RwEngineInstance` +0x20): raster, nearest filter, z test / write off, vertex alpha, src alpha / inv src alpha; z / rhw = `CSprite2d::NearScreenZ` 0xC4B8D8 / `RecipNearClipPlane` 0xC4B8D0 like every game 2D vertex |
| cursor | the same Im2D path, untextured triangles |
| textdraws (game fonts 0–3) | `CFont::SetScale` … `SetFontStyle` in samp's order (14 setters), `PrintString` 0x71A700, `DrawFonts` 0x71A210 (immediately, to keep draw order) |
| sprite textdraws (font 4, `txd:name`) | as samp.dll 0x100B2D10: `CTxdStore::FindTxdSlot` 0x731850, else (`models\txd\NAME.txd`, or `SAMP\samaps.txd` / `SAMP\vehicles128.txd`, when `CFileMgr::OpenFile` 0x538900 finds it) `AddTxdSlot` 0x731C80 + `LoadTxd` 0x7320B0 + `AddRef` 0x731A00; `PushCurrentTxd` / `SetCurrentTxd` / `CSprite2d::SetTexture` 0x727270 / `PopCurrentTxd`; drawn with bilinear filter and `CSprite2d::Draw` 0x728350 (cached per `txd:name`) |
| game text (`GameTextForPlayer`) | `CMessages::AddBigMessage` 0x69F2B0 (style 0..6) with a buffer that outlives the message, exactly what samp does; `CHud` draws it |
| world → screen (nametags, labels) | `CSprite::CalcScreenCoors` 0x70CE30 (both visibility checks); camera position from `TheCamera.m_mCameraMatrix` |
| ped positions | the adapter's `ped_snapshot` (pool-validated handles) |

Text runs are batched until a non-text item needs the screen, so a frame is a handful of Im2D calls. If the
raster cannot be created, UI text falls back to `CFont` font 1 (approximate size).

### 2.1 The UI font

SA:MP's chat, dialogs, nametags and labels use D3DX "Arial" bold. The engine uses **Liberation Sans Bold**
(metric-compatible with Arial; SIL Open Font License 1.1), committed as `assets/fonts/LiberationSans-Bold.ttf`
with its licence `assets/fonts/LiberationSans-LICENSE.txt`, rasterised at start-up with `ab_glyph` (pure Rust,
builds for i686 and wasm32) into a 512-wide alpha atlas: printable ASCII + Latin-1, four sizes (samp's table:
main 14/16/18/20 px by video width, small = main − 2, GUI 20 and 18). Every glyph is also stored with samp's
1-pixel black outline baked in (black at the four offsets under the glyph: `A = f + o(1 − f)`, brightness
`f / A`), so outlined text is one quad per glyph instead of five; the vertex colour tints the core and leaves the
outline black. Textdraws keep the game's own fonts, as in SA:MP.

## 3. Look and behaviour (matches samp-rs's samp-ui spec)

* **Colour codes** `{RRGGBB}`: parsed and stripped as samp.dll 0x100b5fd0 / 0x100b6100 (in-place removal),
  glyph colouring with samp's cursor walk (a code before a blank applies one glyph late; base alpha only until
  the first code). Wrapped lines and labels carry the colour over line breaks.
* **Chat**: 100 entries, page of 10 at (45, 10), line pitch `h + 1`, `name:` prefix in the player colour then the
  text in white (gap 5), player chat split at 72 characters with a 12-character backtrack into a nameless second
  entry, texts cut at 144, control characters → spaces, names over 24 characters dropped; outlined (four
  directions, the 2-pixel eight-offset outline above 1280 px); F7 cycles off / plain / outlined; Page Up / Down
  scroll; `UiOptions`: page size 10..=20 (`/pagesize`), timestamps `[hh:mm:ss]` with the text at
  `width("[19:58:34]") + 50` (`/timestamp`), font size −3..=5 (`/fontsize`, every UI font ±2 px per step); the
  scrollbar (10, 40, 20 × `(h + 1)·page − 60`) shows while scrolled up or while the input line is open.
* **Input line**: T or F6 opens (the T's own character is swallowed), Enter sends (`UiEvent::ChatSubmit`, the text
  or `/command`; samp-rs decides RPC 101 vs 50 and runs local commands), Esc closes, Up / Down walk a 10-entry
  history, Left / Right / Home / End / Backspace / Delete edit, 128 characters; box `min(800, 0.6·W)` wide at
  `(40, chatBottom)`, `14 + 1.5·h0` high; blinking caret.
* **Dialogs**: geometry of samp.dll 0x1006f500 (caption `mainH + 4`, list height 200..400, list width 400..800,
  minimum width 230, two buttons 100×30 at `w/2 − 110` and `w/2 + 10`, edit `14 + 1.5·h0`), background
  0xDC050505, caption 0xFFD2D2D2, body 0xFFA9C4E4 (small font, outlined), selection 0xFFB92228; tab lists with
  up to 4 columns and samp's column widths, style 5 header row; password masked with `*`. Keyboard: Up / Down /
  Page Up / Page Down / Home / End, Enter = button 1, Esc = button 0; mouse: buttons, row select, double click =
  button 1, wheel scrolls. Response = `UiEvent::DialogResponse { id, button, list_item, text }` (list: the
  selected row's first cell without codes; input: the text; ≤ 128 characters).
* **Nametags**: anchor = head (root + 0.7 m) raised `0.0475·d + 0.2`, drawn within the server's distance (70 m
  default), hidden behind buildings and objects unless the server disables the test (`CWorld::GetIsLineOfSightClear`
  0x56A490 with buildings + objects, samp's opcode 06BD flags), all off with `ShowNameTags(0)`; `Name (id)` in the
  player colour (small font, outlined) above samp's bars (health 0xFFB92228 on 0xFF4B0B14, armour 0xFFC8C8C8
  on 0xFF282828, 38.46 px per 100, health 8 px lower when armour shows; geometry of samp.dll 0x1006d0a0).
* **Chat bubbles** (`SetPlayerChatBubble`): ≤ 144 characters split like samp (36 characters, 12 backtrack), drawn
  centred above the nametag at `0.2 + 0.02·(n − 1) + (0.065 + 0.0125·(n − 1))·d` metres over the head (n = lines),
  outlined, within the bubble's distance, until its lifetime (game time) runs out.
* **3D labels**: centred lines at the projected point, small font outlined, codes and `\n`, wrapped at 400 px,
  draw distance per label, optional attachment to a ped, line-of-sight test when the label asks for it.
* **Textdraws**: sprites (font 4) at `position · (W/640, H/448)` sized by `TextSize`, tinted by the letter
  colour; text in samp's `font_params` (scale `W·lw/640`, `H·lh/448·0.5`, wrap, centre size, box, shadow or
  outline, proportional, alignment), samp's hit rectangles for `SelectTextDraw` (hover colour, click →
  `TextDrawClicked(id)`, Esc → `0xFFFF`).
* **Kill list**: 5 rows at `(0.75·W, 0.3·H)` pulled left to fit, names right / left of a 35 px icon box.
* **Scoreboard**: centred panel (60 % × 50 %), id / name / score / ping in the player colours, hover highlight,
  click → `ScoreboardClicked(id)`.
* **Cursor**: shown while a dialog, the scoreboard, textdraw selection or a cursor mode is active.

## 4. Input path

`sa-game` → `samp_input::filter(&mut InputState)` once per main-loop iteration (and in `--window` mode's
`screen::frame`): every platform `InputEvent` becomes a `sa_samp_api::input::WindowEvent` (Windows virtual-key
codes via `sa_platform::win32::key_info`, characters as Unicode, wheel × 120) and goes to
`NativeGame::window_event`: the UI first (`SampUi::handle_event`), then the client's `GameHooks::window_event`.
Consumed events never reach the game's message queue; while the chat line or a dialog is open the held keys,
mouse buttons and mouse motion are cleared too (samp's input lock: typing does not move CJ).

## 5. API changes for the samp-rs pin bump (all additive; `API_VERSION` stays 1)

* New module `sa_samp_api::ui`: trait `SampUi` (`chat_add`, `chat_clear`, `show_dialog`, `set_nametags`,
  `set_label`, `set_textdraw`, `game_text`, `kill_entry`, `set_scoreboard`, `set_cursor_mode`,
  `select_textdraws`, `set_chat_bubble`, `set_options`, `wants_keyboard`, `handle_event`, `poll_events`), types `ChatKind`,
  `ChatMessage` (with `time: [u8; 3]`, the local arrival time), `UiOptions` (page size, timestamps, font size,
  nametag switch / distance / line of sight), `ChatBubble`,
  `DialogStyle` (+ `from_byte`), `DialogSpec`, `NametagAnchor`, `Nametag`, `TextLabel3d`, `TdAlign`,
  `TextDrawSpec`, `ScoreRow`, `CursorMode`, `UiEvent`.
* `Rgba::from_rgba_u32` (wire `0xRRGGBBAA`), `Rgba::from_argb_u32`, `Rgba::to_argb_u32`.
* `GameAdapter::ui(&mut self) -> Option<&mut dyn ui::SampUi>` with a default `None`: the original backend
  compiles unchanged and keeps drawing with D3D9.

How samp-rs uses it (step 11 of game-adapter.md §8, native backend only): in `GameHooks::frame` (any phase
before `After2d`), `if let Some(ui) = game.ui()` → forward chat RPCs 93/101 as `chat_add`, RPC 61 as
`show_dialog`, the per-frame nametag set from its remote players (`NametagAnchor::Ped` with the remote ped's
handle), RPC 36/58 as `set_label`, RPC 59 as `set_chat_bubble`, RPC 134/135 as `set_textdraw`, RPC 73 as `game_text`, RPC 55 as
`kill_entry`, the score list as `set_scoreboard`, RPC 83 as `select_textdraws`, `InitGame`'s nametag flags and
its own `/pagesize` / `/timestamp` / `/fontsize` state as `set_options`; then `poll_events()` →
RPC 101 / 50 / 62 / 83 / 23. samp-rs's local commands (`/q`, `/pagesize`, `/timestamp`, ...) stay in samp-rs.

## 6. Demo and evidence

```
SA_GAME_PRESENT_GAME=1 SA_GAME_SCRIPT_SKIP=1 SA_GAME_STUB=<STD-9 list>,0x680600=0/1,0x7331e0=0/0 \
  target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa --start-in-world --frames 160 \
  --samp-ui-demo --present-dir DIR
```

(`0x680600` `CTaskSimpleInAir::ProcessPed` and `0x7331E0` are stubbed in addition to STD-9's list: both are
reached by the in-world frames on this base and are not registered yet. Some iterations still stop at the
unregistered `CDoor::Process` 0x6F4040 or `CColSphere::IntersectSphere` 0x410090 depending on frame pacing; the
number of playing frames varies from run to run, 2 to 86. Stubbing 0x6F4040 as `0/1` crashes later, so it is
not stubbed.) The demo installs the native adapter
with an empty client and plays samp-rs's part from the frame handler: sample chat (codes, player chat, a wrapped
line), nametags above CJ (and peds within 40 m) with a chat bubble, a label in front of the camera, two text textdraws and two
sprite textdraws (`LD_SPAC:white`, `loadsc2:loadsc2` loaded from `models/txd`), game text, three
kill-list rows; then, through **platform input events** (`samp_input`): T + typing + Enter, a tab list with
headers navigated with Down Down Enter, a password dialog typed and confirmed, the scoreboard clicked; finally
`set_options` (timestamps, page size 14) with F6 and Page Up (scrollbar). Log:

```
samp-ui-demo: event ChatSubmit("hello from the native client")
samp-ui-demo: event DialogResponse { id: 1, button: 1, list_item: 2, text: "MP5" }
samp-ui-demo: event DialogResponse { id: 2, button: 1, list_item: -1, text: "grove4life" }
samp-ui-demo: event ScoreboardClicked(2)
samp-ui-demo: 64 playing frames, 66 overlay frames, atlas uploaded: true, sprites: 2, 4 events
```

Evidence (not committed; Rockstar-derived pixels): `target/samp-ui-evidence/` — `01-chat-nametag-label-textdraws-killlist.png`,
`02-chat-input-typing.png`, `03-tablist-dialog-cursor.png`, `04-password-dialog.png`, `05-scoreboard.png`,
`06-chat-after-events.png`, `07-timestamps-pagesize-scrollbar.png`, `00-contact-sheet.png`, `run.log`. Where the sky is still drawn wrong (STD-9 §15.4)
the frame is not cleared, so text that moved (chat after new lines) leaves earlier frames' pixels there; the demo
holds its echo lines back until the dialogs are done to keep those frames clean. This is not an overlay effect.

## 7. Tests

`cargo test -p sa-samp-ui` (38): colour-code parse / strip / runs (samp's cursor walk), pixel wrapping with
colour carry, player-chat split, chat capacity / truncation / wrap / layout / scrolling / F7 / timestamps /
page size / scrollbar, input history and
submit, edit caret / capacity / password, dialog geometry (message box, input, list, tab list with headers),
list keyboard and mouse navigation, double click, responses, nametag bar geometry (samp's measured values),
nametag projection / raise / culling / rules (switch, distance, line of sight), bubble wrap / raise / expiry, label centring and distance, textdraw params and hit rectangles, sprite sources (`txd:name` rules) and rectangles, kill
list and scoreboard, the full `Ui` state machine (T swallowing, dialogs taking the keyboard, textdraw selection
and Esc, game-text filtering, layer order), the atlas (every glyph present, Arial-like widths, both outlined variants, font size table).
`sa-samp`: conversions of the renderer (1); `sa-game`: platform → window event mapping and the input filter (2).
Clippy `-D warnings`: sa-samp-ui, sa-samp-api, sa-samp on host, i686-unknown-linux-gnu and wasm32-unknown-unknown;
sa-game (all targets) on i686-unknown-linux-gnu and host; sa-core (lib) on i686. `sa-core --all-targets` on i686
fails on a pre-existing `deref_addrof` in `camerasize.rs`'s tests (not touched here).

## 8. Open items

1. **Model previews** (textdraw font 5): need `render_model_preview` (an off-screen RW scene). Not drawn yet.
   Sprite textdraws (font 4) are drawn.
2. **Kill-list weapon icons**: samp draws them with its own GTAWEAPON3 font from samp's files; the engine shows a
   short label in the icon box. Needs the font from samp.saa through `FileProvider`, or a bitmap set.
3. **Nametag AFK icon** (SAMPAUX3 glyphs): shown as "AFK" text. Nametags of drivers of RC vehicles anchor at
   the vehicle in samp; samp-rs can pass `NametagAnchor::World` for them.
4. **Chat log file**: samp-rs keeps it.
5. **The DXUT look** of buttons, edit boxes and the scrollbar is approximated with flat rectangles (samp's GUI
   skin texture is samp's own file).
6. **Menus** (`ShowMenu`, GTA's `CMenuSystem`) stay on `hud::Menus`, not this UI.
7. **wasm32**: the UI crate and the backend build; `sa-game` itself does not target wasm32 yet.

## 9. Proposed ROADMAP / WORKLOG rows (for the main session)

| ID | Task | Status | Acceptance | Notes |
|---|---|---|---|---|
| SAMP-UI | Engine-drawn SA:MP UI (chat, input, dialogs, nametags, labels, textdraws, game text, kill list, scoreboard, cursor) for the native backend | in-progress | samp-rs on the native backend shows its UI through `GameAdapter::ui`, look and behaviour as samp 0.3.7 | 2026-11-12: `crates/sa-samp-ui` (state + layout + input → draw list, 38 tests), `sa-samp::overlay` (draws through `CSprite2d::DrawRect`, RW Im2D glyph quads over one RW raster, `CFont`, `CMessages::AddBigMessage`, `CSprite::CalcScreenCoors`), API `sa_samp_api::ui` (additive), Liberation Sans Bold (OFL) atlas, `sa-game --samp-ui-demo` with platform-input-driven chat / dialogs / scoreboard (evidence `target/samp-ui-evidence/`). `FramePhase::Present` dispatch in `Game::Idle`. Open: model previews, weapon icons, AFK icon (docs/samp/overlay.md §8) |

WORKLOG entry (proposal): "SAMP-UI: engine-drawn SA:MP overlay. New crate sa-samp-ui (renderer-independent UI
state, layout and input handling following samp-rs's samp-ui spec), sa-samp overlay module (draws through
ported CSprite2d / RW Im2D / CFont / CMessages, world → screen with CSprite::CalcScreenCoors), additive API
module sa_samp_api::ui + GameAdapter::ui, Liberation Sans Bold (OFL 1.1) glyph atlas with baked outline,
sa-game platform input → UI (samp_input) and --samp-ui-demo. Verified: 38 + 1 + 2 host tests; clippy -D warnings
host / i686 / wasm32 (UI, API, backend), host / i686 (sa-game); demo run on the user's install (release i686,
--start-in-world, STD-9 stubs + 0x680600, 0x7331E0): 4 UI events from platform input, PNGs in
target/samp-ui-evidence/."
