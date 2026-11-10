# Selection and pointer picking: scope decision

Decision paper for [#92](https://github.com/dfadler/Burl/issues/92) (`OPTION-COVERAGE.md` Gap 9). It writes no feature code. The user picks an option; the chosen one becomes a ticket and, unless it is full parity, a row in `DEVIATIONS.md` (policy: #10). All C++ references are to `src/` at 0.57.

**Status: historical.** Option B2 was chosen and has landed (#119-#124). Sections 2 to 7 describe `master` as it was before that stack; in particular `--selection-colour` is no longer unused (see `crates/burl/src/frame.rs` and `crates/burl-render/src/labels.rs`).

## 1. What Gource does

There are two ways to select, one shared effect, and one hover effect.

### 1.1 Picking (`Gource::mousetrace`, gource.cpp:1881-1987)

Runs once per frame at the top of `draw` (2415-2427), only when `!hide_mouse && cursor.isVisible()`. Otherwise it clears `hoverUser`/`hoverFile` (2419-2426).

1. Mouse to world (1884-1892): `x = -(mx*2 - W)/H * cam.z + cam.x`, `y = (1 - 2*my/H) * cam.z + cam.y`. This assumes the fixed FOV and the mouse position in display pixels.
2. Users first, unless `hide_users` (1897-1909): `userTree->getItemsAt(point)` returns the *whole leaf* with no point test (HANDOFF gotcha). The first user in `std::set<QuadItem*>` order, which is **pointer-address order**, that is not `isFading()` and whose `quadItemBounds` contains the point wins.
3. Files only if no user was hit and not `hide_files` (1911-1932): `dirNodeTree->getItemsAt(point)`, then for each dir in that set (address order), each file, `!isHidden() && overlaps(point)` (file.cpp:100: a box of `size*0.5` by `size*0.5*graphic_ratio` around `pos + dir->pos`). The `break` only leaves the inner loop, so the *last* dir in the set that has a hit decides.
4. Hover bookkeeping (1935-1973): the file branch is tested first, but it can only be non-null when no user was hit, so a user wins. Changing target calls `setMouseOver(false/true)` on old and new. Nothing under the mouse clears both.
5. Click (1975-1985): if `mouseclicked`: user hit gives `camera.lockOn(false); selectUser(hoverUser)`; file hit gives `camera.lockOn(false); selectFile(hoverFile)`; else `selectBackground()`.

`mouseclicked` is set on left-button-down (gource.cpp:490) and cleared at the end of `draw` (2759-2760). If `mousetrace` did not run that frame (cursor hidden), the click is dropped.

### 1.2 Input handlers

- `mouseMove` (384-396): ignored if `hide_mouse`, `disable_input`, no log, or `grab_mouse` (dragging). Otherwise sets `mousepos` and `mousemoved`.
- `mouseClick` (436-499): same guards. Left-down sets `mouseclicked` and, if `canSeek()`, the slider click. Middle toggles the camera mode (476-479). Right grabs the mouse (482-485).
- `selectBackground` (545-557): only if `cursor.leftButtonPressed()` *at frame time*: `selectUser(0)`, `manual_camera = true`, `mousedragged = true`, `grabMouse(true)`. A press and release inside one frame therefore neither deselects nor drags.
- `selectUser` (560-587): no-op if already selected. Deselects the current file and user, then `setSelected(true)` and `camera.lockOn(true)` only if `track_users`. A null user does `camera.lockOn(false)`.
- `selectFile` (590-615): deselects user and file, `setSelected(true)`. No camera lock.
- Other entry points to `selectUser`: Tab, `selectNextUser` (618-653, in tag-id order, skipping inactive and non-opaque users); `--follow-user` (1389-1396); deselect when the user fades (1377-1378); and when the user or file is removed or a section resets (894-917, 967-972, 1045-1050).

### 1.3 What "selected" changes

| Where | Effect |
|---|---|
| `RUser::updateFont` (user.cpp:305-312) | Selected user label uses an unscaled 18 pt font (not `scaled_user_font_size`), with drop shadow. |
| `RUser::getNameColour` (user.cpp:330) | `selection_colour` beats `highlight_colour` beats the hash colour. |
| `RUser::getColour` (user.cpp:334) / `RFile::getColour` (file.cpp:153) | Sprite is drawn white while selected. |
| `Pawn::nameVisible` / `RUser::drawNameText` (pawn.cpp:112, user.cpp:387) | A selected label is always visible at full user alpha, regardless of `name_interval` or `hide_usernames` for that user's second draw. |
| `RFile::drawNameText` (file.cpp:281-288) | Selected file: 18 pt `file_selected_font` in `selection_colour` (file.cpp:54-58, a static created once), name not extension, always visible. |
| `RFile::calcScreenPos` (file.cpp:269) | Selected file label sits 2 instead of 1 unit below. Computed even under `hide_filenames` (gource.cpp:2457). |
| `RDirNode::drawNames` (dirnode.cpp:988) and `draw` (2502, 2536-2540) | Selected pawns are skipped in the normal pass and drawn again last, over everything. |
| `updateCamera` (1507-1512) | In track mode, the camera bounds are the selected user and/or file position. The Rust sim already does this for the user (`sim.rs:2170`). |
| `Pawn::mouseover` | **Set and never read** (`grep mouseover` finds only pawn.cpp and the slider). Hover has no effect other than the text box below. |

### 1.4 Hover text box (gource.cpp:2673-2692, textbox.cpp)

Drawn after the slider. A file that is hovered and not selected shows two lines: the name, then the path with the leading `/` removed (if any is left). A hovered, unselected user shows its name. Colour is the pawn's `getColour()`; brightness 0.5. Font is 18 pt times `font_scale` (gource.cpp:142). `setPos(mousepos, true)` puts the corner above-left of the cursor and flips to stay in the window (textbox.cpp:`setPos`). Drawing: a black quad at alpha/3 offset (3,3), the coloured quad, then white text rows spaced `font size + 4` (textbox.cpp:`draw`). It is the only hover feature.

## 2. What Burl had before #119-#124 (historical)

- `selected_user` lives in `burl-sim` (`sim.rs:300`). `select_user` ports only the camera lock-on (`sim.rs:1216`); fade-deselect and follow-select are ported (`1165-1207`, golden `small-repo-follow`). There is no `selected_file` and no `selectedUser` in `SceneView`/`UserView`.
- `Command::DragStart` ports `selectBackground` (`command.rs:195`). `Pointer::button` (`interactive/input.rs:173`) sends it for **every** left press. Burl therefore deselects and drags even when the press is on a user or file: there is no picking, and the checklist line "clicking a user selects it" cannot pass (`INTERACTIVE-CHECKLIST.md:35`). `Pointer` has no position at all: `CursorMoved` is not handled, only relative `DeviceEvent::MouseMotion` (`interactive.rs:459`).
- Not ported: Tab / `selectNextUser` (no Tab key row in `input.rs`), middle-click camera toggle, `SelectedFile`, the text box (`overlays.rs:5`, `labels.rs:5` say so), the selected label pass.
- `--selection-colour` is parsed into `Settings::selection_colour` (`burl-conf/src/settings.rs:212`) and used nowhere else. `--follow-user` and `--camera-mode track` are refused by the CLI while `sim-follow-track` is in flight (OPTION-COVERAGE Gap 3).
- Building blocks that exist: `QuadTree::visit_items_at` (`burl-util/src/quadtree.rs:168`), which has the same whole-leaf semantics as C++; the per-frame user tree (`sim.rs:1015`) and dir tree (`interact_dirs`, `sim.rs:2127`), both local to a step and not retained; `Font`/`FontManager::draw` and batches for quads (`burl-render`).

**Important:** the selected *rendering* is not windowed-only. Once `--follow-user` lands in `-o` export, `selectUser` runs in the export path and the oracle draws the selected user white with a 18 pt label in `selection_colour` (1.3). So the "selected pawn look" has to exist for export parity regardless of the pointer. Picking, hover and the text box are windowed-only.

## 3. Pieces and their dependencies

```
P1 selected look   (view flags, label+colour changes)   needs: sim exposes selected user/file
P2 selection state (selected_file, select_file, Tab)    needs: nothing (sim only)
P3 picking         (mouse->world, user/file hit test)   needs: mouse position (P5), a view of the scene
P4 click dispatch  (user/file/background)               needs: P3, P2, replaces DragStart-on-every-press
P5 pointer input   (CursorMoved, grab and hide rules)   needs: windowed loop, interactive.rs
P6 hover text box  (quads+text near cursor)             needs: P3, quad rendering in the overlay
```

| Piece | Files | Size | Notes |
|---|---|---|---|
| P1 | `burl-sim/src/view.rs` (`selected` on `UserView`/`FileView`), `sim.rs` (fill), `burl-render` `scene.rs` (sprite white), `labels.rs` (18 pt, colour, always-on, drawn last, y offset), `NameSettings` gets `selection_colour`, `burl/src/export.rs` + `frame.rs` plumb the setting | ~150 LOC | Golden-testable once follow-user exists. A second font size (18) must be grabbed. |
| P2 | `sim.rs` (`selected_file`, `select_file`, clear-on-removal, section reset, camera focus bounds incl. file), `command.rs` (`SelectUser`, `SelectFile`, `NextUser`) | ~120 LOC | Tab needs `tagusermap` order (tag id), which `UserId` may not preserve; check. |
| P3 | new pure `pick(view, camera, mouse, W, H, settings) -> Option<Pick>` in `burl-render` or `burl-sim` | ~80 LOC | Linear scan of `SceneView` is enough for a window (hundreds of pawns). Using the real quadtrees would mean retaining them in `Sim` and `Sim` owning the mouse. See 4 on tie-break. |
| P4 | `command.rs`, `input.rs` (`Pointer::button` stops sending `DragStart` unconditionally), `interactive.rs` | ~60 LOC | Must move the decision from event time to frame time (see 1.2 `selectBackground` quirk) or accept a deviation. |
| P5 | `interactive.rs`, `input.rs` | ~50 LOC | `CursorMoved` carries physical pixels; C++ uses SDL window coordinates. Retina must be checked (G3, `--high-dpi`). Ignored while grabbed/hidden/disabled. |
| P6 | new `burl-render` `textbox.rs`, hook in `frame.rs` | ~120 LOC | Needs an untextured alpha quad batch (the key legend already draws them), text at 18 pt x `font_scale`. **Touches `frame.rs`**, which other sessions are changing now. |

Total for everything: roughly 600 LOC in 8 or 9 files and 3 crates, mostly new code with little to rewrite.

## 4. Deviations that are hard to avoid

- **Overlap tie-break.** C++ picks by `std::set<QuadItem*>` order, i.e. heap addresses. It is not reproducible even between two Gource runs. Burl must define an order (proposed: user list order, then for files, dirs and files in sim creation order). This is a Change whatever option is chosen, and cannot be golden-compared on overlaps.
- **Dir culling.** C++ file picking only finds files whose *dir quad bounds* contain the point. A linear scan over files finds more. Matching it needs the dir tree. Proposed: scan dirs by their bounds first, same as C++.
- **Event time versus frame time.** If P4 is done as `mouseclicked` (frame time), a very quick click is missed exactly as in Gource; if done at event time it is not. Proposed: event time for the *selection* (the visible result is the same) and note it.

## 5. Options

### Option A: Document as a Gap; no pointer selection (smallest)

`--selection-colour` stays accepted and stored (not refused: refusing a valid Gource flag breaks config files and scripts that only set it and it is harmless in `-o` without follow). Stop sending `DragStart` on clicks that would have hit a pawn only if picking existed, i.e. leave as is. Update the checklist so the hover/select lines are not expected.

- **Size:** ~20 lines of docs and one test. No code.
- **Risk:** none technically. Visible gap: the Gource windowed UX of hover names and click-select is missing, and the checklist (#94) cannot be signed as written. Once `--follow-user` lands, `selection_colour` is silently ignored on a selected user, which makes that flag *wrong* instead of just unfinished unless P1 is added.
- **Tests:** one Burl-only test `deviation_g10_selection_not_ported` (e.g. a left press never produces a command other than `DragStart`; `selection_colour` parses). Manual: nothing new.
- **Register rows:** `G10` Gap: *Gource selects the hovered user or file on click and shows a hover text box; Burl treats every left press as a background drag and draws no text box.* Fix trigger: first user report, or the parity release decides pointer selection is required. `G11` Gap: `--selection-colour` stored, no effect (drop it if A+ is chosen). Plus a note under Gap 3 about follow-user labels.

### Option A+: Option A plus the selected look (P1 and P2 minus the file)

Renders `selectedUser` the way Gource does (white sprite, 18 pt label in `selection_colour`, drawn last), driven only by what already selects users in the sim: `--follow-user` and fade-deselect. No pointer, no Tab, no files, no text box. This is the part that matters for export parity and that `sim-follow-track` will need anyway.

- **Size:** ~150 LOC, 5 files, no `interactive.rs` or `frame.rs` edits (the export `Frame` builder is shared; one small plumb for the setting).
- **Risk:** low. Rendering of a selected user is deterministic.
- **Tests:** **golden-testable**: add a render fixture of `small-repo-follow` to the visual gate (the sim golden exists), SSIM against the oracle at the frames where bob is selected. Unit test in `labels.rs` for the colour, size, order and the unscaled font. Sabotage check: removing the white sprite must fail the visual gate.
- **Register rows:** `G10` as in A (pointer picking, hover box, Tab, file selection not ported). `--selection-colour` is now wired, so no `G11`.

### Option B: Full parity, in two steps

B1 = A+ plus pointer selection of users and files and Tab (P1-P5). B2 = B1 plus the hover text box (P6).

- **Size:** B1 ~450 LOC, 8 files, 3 crates (`burl-sim`, `burl-render`, `burl`). B2 adds ~120 LOC and a `frame.rs` change.
- **Risk:** medium. Touches `interactive.rs`, `input.rs`, `frame.rs`, all being changed now by other sessions (avatars, windowed fixes, slider) so it conflicts unless it waits for them. The pointer needs HiDPI coordinate handling that can only be checked by hand. Camera lock-on in track mode interacts with picking.
- **Tests:** the pure pieces are Burl-only unit tests: mouse-to-world formula (compare numbers computed from the C++ formula), hit tests on constructed `SceneView`s (user wins over file, fading users skipped, hidden files skipped), `Pointer` behaviour. The oracle cannot be compared on picking today: the dump fork is headless and takes no pointer. A real oracle test needs a fork patch that injects scripted `(frame, x, y, button)` events into `mouseMove`/`mouseClick` and dumps `hoverUser`, `selectedUser`, `selectedFile` (about a day of work, medium risk, but then sim-state *and* visual gates cover clicks). The alternative, `xdotool` against Xvfb, is not deterministic about which frame an event lands in. The text box and everything about real cursor feel (cursor hiding, grab, HiDPI, flipping at the window edge) are **manual-only**: add rows to `INTERACTIVE-CHECKLIST.md` for macOS Retina, non-Retina and Linux X11.
- **Register rows:** `D13` Change: overlap tie-break is deterministic (Gource: pointer order). `D14` Change or Gap: click selects at event time, not frame time. Optional `G11` Gap if B1 only: no hover text box (`P6` deferred). `G10` is not needed.

## 6. Comparison

| | A | A+ | B1 | B2 (full) |
|---|---|---|---|---|
| `--selection-colour` effective | no | yes (follow) | yes | yes |
| Click selects, Tab | no | no | yes | yes |
| Hover text box | no | no | no | yes |
| LOC / files | docs | ~150 / 5 | ~450 / 8 | ~570 / 9 |
| Conflicts with in-flight windowed work | none | none | high | high |
| Oracle-testable | n/a | yes (follow fixture) | no, unless fork patch | no, unless fork patch |
| Manual checklist rows | remove | remove | 3 | 4 |
| Register rows | G10, G11 | G10 | D13, D14, G11 | D13, D14 |

## 7. Recommendation

Do **A+ now, B later**:

1. Take A+ because the selected-user look is needed for `--follow-user` in export, is deterministic and golden-testable, and does not touch the windowed files other sessions are changing. File it as its own ticket, and sequence it after `sim-follow-track` (it can start from the existing `small-repo-follow` golden).
2. Record `G10` (no pointer picking, Tab, file selection or hover text box) with the fix trigger "first user report or parity-release decision", and remove the checklist line "Hover shows names; clicking a user selects it" (or move it to a later release), so #94 can be signed off.
3. Do not refuse `--selection-colour`: it is a valid Gource flag and becomes live under A+.
4. Revisit B once the windowed work has settled (`interactive.rs`/`frame.rs` stable, the slider landed, which also needs the pointer position). Then do B1 with the scripted-pointer fork patch, and B2 last.

Decision wanted from the user: A, A+ (recommended), B1 or B2; and whether to accept `D13` (deterministic tie-break) if B is chosen.

## 8. Decision

The user chose **full parity** (B2: selected look, pointer picking of users and files, Tab, and the hover text box). The implementation is a separate ticket and PR sequence, to start once the windowed path (`interactive.rs`, `frame.rs`) is stable; suggested order: P1+P2, then P3-P5, then P6, with the scripted-pointer oracle fork patch ahead of P3 so picking has a real oracle test. Rows `D13` (deterministic overlap order) and `D14` (selection at event time) still need the user's explicit OK before they merge.

Row numbers: `D12` is taken by the 0.58.0 version-line row in the release PR (#114), so the rows proposed here are `D13` (deterministic overlap order) and `D14` (selection at event time).
