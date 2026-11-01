# Rust architecture map (where an implementer starts)

Updated 2026-10-31 for the final decisions; see docs/rust-rewrite/discovery/LEDGER.md Decisions. The binary crate is `burl` (`crates/burl`); the internal library crates keep their `gource-*` names below, and renaming them is still undecided.

Synthesis of the other discovery docs; nothing here is new C++ research. Doc abbreviations: APP = app-core.md, SCN = scene-model.md, SET = settings-cli.md, LOG = log-formats.md, COR = core-lib.md, OUT = output-pipeline.md, DEP = deps-and-rust-mapping.md, TIM = timing.md, TST = tests-and-golden-strategy.md, BLD = build-and-goldens.md. Section refs are the doc's own headings ("T2.x" = its Tick 2 additions). Where docs conflict, TIM wins on timing (TIM sec. 6).

Target contract (LEDGER top): same CLI/config/log API, same parsed commits and simulation state as Gource, visually equivalent (not pixel-identical) rendering.

## 1. Workspace layout and file-to-module map

### 1.1 Crates

Dependency arrows point downward only. `gource-sim` never imports a renderer, window, GL, or winit/glutin type (APP T2.7: draw consumes no rand and mutates no sim state, so a headless sim is possible).

| Crate | Role | Depends on | Contents |
|---|---|---|---|
| `gource-util` | pure helpers | glam | `prng`, `strhash` (stringHash, vec2Hash, colourHash), `math` (normalise, rotate_vec2, PI=3.14159265 const), `bounds` (Bounds2D), `quadtree`, `utf8` (replace_invalid port), `regex` wrapper, `timeparse` (parseDateTime, mktime_utc) |
| `gource-conf` | config + CLI model | util | `conffile` (ConfFile), `args` (parseArgs), `settings` (typed `DisplaySettings`, `GourceSettings`, import/validate), `help` (static text) |
| `gource-formats` | log parsers | util, conf (filters only) | `seeklog`, `commitlog` trait + `RCommit`/`RCommitFile`, nine parsers, `logmill` |
| `gource-sim` | simulation state | util, formats, conf | `dir_tree` (arena), `file`, `user`, `action`, `pawn`, `camera` (ZoomCamera), `gource` (Gource logic, no draw), `shell` (multi-section state machine), `dump` (debug state dump) |
| `gource-render` | trait + CPU-side batching | sim, util | `Renderer` trait, `scene` (builds vertex batches from sim state), `text` layout, `ui` overlays, `spline` (SplineEdge geometry), `bloom` |
| `gource-gl` | backend impl | render, winit, glutin, glutin-winit, glow | window/context, shaders (`#version 150`), VBOs, textures, FreeType atlas, `PpmWriter`, PNG |
| `burl` (bin, `crates/burl`) | app | all | `main` flow, `GourceShell` loop, input mapping, exit/err handling |

Rationale for the split: the parity gates (parser golden, sim dump) must run with only `gource-formats` / `gource-sim` compiled and no GL or windowing available, matching TST sec. 4 (the C++ sim is not headless; the Rust one must be).

### 1.2 Module tree

```
crates/
  gource-util/src/{lib,prng,strhash,math,bounds,quadtree,utf8,regex,timeparse}.rs
  gource-conf/src/{lib,conffile,args,settings_display,settings_gource,help}.rs   # help.rs = include_str!("help-short.txt"/"help-long.txt")
  gource-formats/src/{lib,seeklog,commit,logmill}.rs
  gource-formats/src/formats/{git,gitraw,custom,svn,hg,bzr,cvs_exp,cvs2cl,apache}.rs
  gource-sim/src/{lib,pawn,dir_tree,file,user,action,camera,gource,shell,dump}.rs
  gource-render/src/{lib,renderer,scene,text,ui,spline,bloom,key,slider,textbox,caption}.rs
  gource-gl/src/{lib,context,shader,vbo,texture,font,ppm,png}.rs  + shaders/*.vert|*.frag (GLSL 150)
  burl/src/{main,app,input,shell_loop}.rs
```

### 1.3 C++ file to Rust module

Class column: S = sim-state (parity-gated), R = render, U = UI/input only, I = I/O. "Not built" files are listed in COR sec. 0 / T2 (f).

| C++ file | Major class / content | Rust module | Type | Class |
|---|---|---|---|---|
| src/main.cpp | program flow (SET sec. 1) | `burl/main.rs` | `fn main`, `run()` | I |
| src/gource_shell.cpp | `GourceShell : SDLApp`, multi-section state machine (SET T2.1) | `gource-sim/shell.rs` (state machine) + `burl/shell_loop.rs` (frame loop, transition texture) | `Shell` | S + R |
| src/gource.cpp / .h | `Gource` (logic, commit ingestion, camera drive, processCommit, writeCustomLog, draw) | `gource-sim/gource.rs` (logic) + `gource-render/scene.rs` (draw) + `burl/input.rs` (key/mouse map, APP T2.2) | `Gource`, `SceneBuilder` | S / R / U |
| src/gource_settings.cpp / .h | `GourceSettings`, help text, `commandLineOption`, import | `gource-conf/settings_gource.rs`, `help.rs` | `GourceSettings` | S (values) |
| src/core/settings.cpp / .h | `SDLAppSettings` display half, `parseArgs`, `parseDateTime` | `gource-conf/{args,settings_display}.rs`, `gource-util/timeparse.rs` | `DisplaySettings`, `parse_args` | I |
| src/core/conffile.cpp / .h | `ConfFile`, `ConfSection`, `ConfEntry` | `gource-conf/conffile.rs` | `ConfFile` | I |
| src/logmill.cpp / .h | `RLogMill` (thread, format detect) | `gource-formats/logmill.rs` | `LogMill` | I |
| src/formats/commitlog.cpp / .h | `RCommitLog`, `RCommit`, `RCommitFile` | `gource-formats/commit.rs` | `trait CommitLog`, `Commit`, `CommitFile` | I (feeds S) |
| src/formats/{git,gitraw,custom,svn,hg,bzr,cvs-exp,cvs2cl,apache}.cpp | nine parsers (LOG sec. 6) | `gource-formats/formats/*.rs` | one `struct` each implementing `CommitLog` | I |
| src/core/seeklog.cpp | `SeekLog`, `StreamLog` | `gource-formats/seeklog.rs` | `trait LineSource`, `SeekLog`, `StreamLog` | I |
| src/dirnode.cpp / .h | `RDirNode` (tree, forces, colour, label, bloom VBO) | `gource-sim/dir_tree.rs` (tree+forces) + `gource-render/scene.rs` (label, edge, bloom emit) | `DirTree`, `DirNode` | S / R |
| src/file.cpp / .h | `RFile : Pawn` | `gource-sim/file.rs` | `File` | S |
| src/user.cpp / .h | `RUser : Pawn` | `gource-sim/user.rs` | `User` | S |
| src/action.cpp / .h | `RAction`, `Create/Remove/ModifyAction` | `gource-sim/action.rs` | `Action`, `ActionKind` | S |
| src/pawn.cpp / .h | `Pawn` base | `gource-sim/pawn.rs` (shared struct embedded in File/User) | `Pawn` | S |
| src/zoomcamera.cpp / .h | `ZoomCamera` | `gource-sim/camera.rs` | `ZoomCamera` | S (CAM) |
| src/spline.cpp / .h | `SplineEdge` | `gource-render/spline.rs` | `SplineEdge` | R |
| src/bloom.cpp / .h | `bloombuf` | `gource-render/bloom.rs` | `BloomBatch` | R |
| src/key.cpp / .h | `FileKey`, `FileKeyEntry` | `gource-render/key.rs` (counts are updated from sim via `inc/dec` events) | `FileKey` | R |
| src/slider.cpp / .h | `PositionSlider` | `gource-render/slider.rs` | `PositionSlider` | U |
| src/textbox.cpp / .h | `TextBox` | `gource-render/textbox.rs` | `TextBox` | U |
| src/caption.cpp / .h | `RCaption`, caption-file load | `gource-render/caption.rs` (loader in `gource-formats` or conf) | `Caption` | U |
| src/core/display.cpp | `SDLAppDisplay`, GL context, projection, unproject | `gource-gl/context.rs` (winit window plus glutin context behind the `GlContext` trait) | `GlContext` | R |
| src/core/sdlapp.cpp | `SDLApp` frame loop, errors, resource dirs | `burl/shell_loop.rs` (event pump is a winit 0.30 `ApplicationHandler`, not a poll loop: inversion of control changes the loop structure), `gource-util` (err format) | `run_loop` | I |
| src/core/fxfont.cpp | `FXFont`, `FXFontManager` | `gource-gl/font.rs` | `FontAtlas` | R |
| src/core/shader.cpp, shader_common.cpp | `Shader`, preprocessor | `gource-gl/shader.rs` (preprocessor dropped, COR sec. 17) | `ShaderProgram` | R |
| src/core/texture.cpp, vbo.cpp | `TextureResource`, VBO helpers | `gource-gl/{texture,vbo}.rs` | `Texture`, `Vbo` | R |
| src/core/ppm.cpp | `PPMExporter` | `gource-gl/ppm.rs` | `PpmWriter` | I |
| src/core/png_writer.cpp | PNG screenshot | `gource-gl/png.rs` | `write_png` | I |
| src/core/quadtree.cpp | `QuadTree`, `QuadItem` | `gource-util/quadtree.rs` | `QuadTree` | S |
| src/core/bounds.h | `Bounds2D` | `gource-util/bounds.rs` | `Bounds2D` | S |
| src/core/frustum.cpp, plane.cpp | `Frustum` | `gource-render/scene.rs` (cull) | `Frustum` | R |
| src/core/vectors.cpp, pi.h | normalise, rotate | `gource-util/math.rs` | fns | S |
| src/core/stringhash.cpp | stringHash, colourHash, vec2Hash | `gource-util/strhash.rs` | fns + seed state | S |
| src/core/regex.cpp | `Regex` | `gource-util/regex.rs` | `Regex` | S/I |
| src/core/timezone.cpp | TZ hack, `mktime_utc` | `gource-util/timeparse.rs` | fns | I |
| src/core/logger.cpp | `Logger` | `log` + `env_logger`-like own impl, tag format kept (COR sec. 17) | `Logger` | I |
| src/core/mousecursor.cpp | `MouseCursor` | `gource-render` (drawn after PPM dump, TIM sec. 1) | `Cursor` | U |
| src/core/resource.cpp | resource lookup | `gource-gl` (paths in COR sec. 3) | `ResourceDirs` | I |
| src/core/ui/*, tga.cpp, timer.cpp | not built / unused by Gource (COR sec. 0) | none (TGA only if `image` crate lacks bloom.tga need; shader bloom ignores it, OUT sec. 4) | - | - |
| src/core/utf8/, tinyxml/, ftgl/ | `core/utf8/` and `core/ftgl/` are header-only (not in Makefile.am sources). `src/tinyxml/*.cpp` is built only `if USE_BUNDLED_TINYXML`, else system tinyxml (corrected tick 4: Makefile.am conditional; every other .cpp in Makefile.am `sources` plus `src/main.cpp` has a row above, and no unbuilt file is listed as required) | `gource-util/utf8.rs`; `roxmltree`/`quick-xml` for svn/cvs2cl; ftgl replaced by FreeType binding | - | I/R |
| src/test/*.cpp | datetime, regex Boost tests | `cargo test` in `gource-util` (TST sec. 1) | - | - |

## 2. Core data model

### 2.1 Commit types (formats crate)

```rust
pub struct CommitFile { pub filename: Vec<u8> /* UTF-8 filtered, leading '/' added */, pub action: String /* "A","D", else modify */, pub colour: Vec3 }
pub struct Commit { pub timestamp: i64, pub username: String, pub files: Vec<CommitFile> /* insertion order */ }
```

Rules (LOG sec. 2): ctor prepends `/` if missing, no other normalisation; `files` keep log order; zero-file commits skipped by `is_valid`; colour from `fileColour` (extension hash) unless custom log supplies one. Keep `username`/`filename` as `String` after the `utf8::replace_invalid` port (LOG sec. 2 UTF-8 row; `from_utf8_lossy` is NOT equivalent, COR T2 (d)); keep a `Vec<u8>` view for pre-filter byte-regex matching because file filters run on the raw filename (LOG sec. 2 addFile row).

### 2.2 Directory tree: recommend arena with indices

**Recommendation: `DirTree { nodes: Vec<Option<DirNode>>, free: Vec<u32>, root: DirId, by_path: BTreeMap<String, DirId> }`, with `DirId(u32)` and `FileId(u32)` newtypes. Not `Rc<RefCell>`.**

Why (SCN sec. 3.2, 3.5, 10):
- The algorithms mutate across parent, child and sibling at once: `addNode` erases from `children` while recursing into the moved child; `addFile` may create a new parent above the current root and `Gource::addFile` then walks `root = root.getParent()`; split-to-intermediate-dir moves several children into a new node (SCN 3.2). With `Rc<RefCell>` that is a runtime-borrow-panic minefield; with indices each step is `tree.nodes[id]` access with short borrows, and a `fn children_of(&self, id) -> &[DirId]` clone-free read.
- Force application reads neighbours' `pos`/`dir_area` while writing the current node (SCN 3.5, hazard 6: "stale child dir_area read by parent"). Indices let the code copy the few scalars it needs and then write, preserving read-before-write ordering exactly. Pointer graphs make that order implicit and easy to change by accident.
- The quadtree stores items, not pointers (`QuadItem` becomes `Item::Dir(DirId) | Item::User(UserId)`), so no lifetime coupling. The C++ `std::set<QuadItem*>` pointer ordering never reaches sim state (LEDGER tick 2; COR T2 (c)), but visit order (insertion, TL/TR/BL/BR, rebuilt per tick) does, so store item ids in a `Vec` per leaf in insertion order.
- Dropping is deterministic: `gGourceDirMap` unregistering on delete (SCN 3.2) becomes `by_path.remove` inside `remove_node`.
- Freed ids go to a free list; ids are never exposed to output, so reuse is safe. Never `swap_remove` in `children`/`files`: they are insertion-ordered lists whose order drives float accumulation and rand() consumption (SCN sec. 0 table, hazard 4). Use `Vec::remove` (O(n) is fine) or `retain`.

Quirk to carry: the C++ ctor shadows `parent` so `parent` is always 0 after construction and set only by `addNode -> setParent` (SCN 3.1). In Rust there is no ctor-time parent argument at all; `parent: Option<DirId>` starts `None`. `pos` also starts (0,0); the parent-branch in the ctor is dead code.

```rust
pub struct DirNode {
  abspath: String /* trailing '/' */, path_token: String, path_token_offset: usize,
  parent: Option<DirId>, children: Vec<DirId>, files: Vec<FileId>,
  col: Vec4, pos: Vec2, spos: Vec2, accel: Vec2, prev_accel: Vec2,
  dir_area: f32, file_area: f32, dir_radius: f32, parent_radius: f32,
  visible: bool, position_initialized: bool,
  since_node_visible: f32, since_last_file_change: f32, since_last_node_change: f32,
  depth: i32, visible_count: i32,
  quad_bounds: Bounds2D, /* render-only fields (in_frustum, projected pos) live in gource-render, keyed by DirId */
}
```

Files (`Vec<Option<File>>` arena, `FileId`) and users (`Vec<Option<User>>` arena, `UserId`) follow the same scheme. `File.dir: Option<DirId>`; `Action { source: UserId, target: FileId }` replaces raw pointers, and `User::file_removed(FileId)` (SCN sec. 4 Delete) drops queued and active actions that target the id. All f32 (SCN sec. 10 hazard 1: mirror per-expression f64 literal narrowing, e.g. `PI` is the f64 literal `3.14159265`, not `std::f64::consts::PI`).

### 2.3 Other structs

| C++ | Rust shape | Notes (source) |
|---|---|---|
| `Pawn` | `struct Pawn { pos, shadow_offset, name, namewidth, accel, speed, elapsed, fadetime=1, nametime=5, name_interval, namecol, shadow, hidden, tagid, selected, mouseover, size, graphic_ratio, dims }` embedded as a field (`file.pawn`, `user.pawn`) | SCN sec. 2. Composition, not a trait object. Default-constructed glm vectors are zero by definition (`GLM_FORCE_CTOR_INIT`, core/vectors.h:34), so `accel`/`dest`/`touch_colour` start at zero (corrected tick 4: not UB; verification-tick3-scene.md row 1). Only `QuadItem::node_count` is truly uninitialised until `addItem` sets it; use 0. `graphic` texture handle moves to render side keyed by id |
| `RFile` | `struct File { pawn, file_colour, touch_colour, dir, removed_timestamp: i64, forced_removal, expired, removing, fade_start: f32 /* -1 sentinel */, last_action, dest, distance, path, fullpath, name, ext }` | SCN sec. 4. Keep `fade_start > 0.0` test (0.0 means not fading). New files hidden=true |
| `RUser` | `struct User { pawn, actions: VecDeque<ActionId>, active_actions: VecDeque<ActionId>, last_action, action_interval=0.2, usercol, highlighted, ... }` | SCN sec. 7. FIFO order matters |
| `RAction` | `struct Action { kind: ActionKind /* Create, Remove, Modify{colour} */, source, target, timestamp: i64, t: f32, progress, rate }` | SCN sec. 5. Arena `Vec<Option<Action>>` or inline in the user queues; fire `target.touch` when `progress == 0.0` |
| `ZoomCamera` | `struct ZoomCamera { pos, dest, target, up=(0,-1,0), lockon, speed, lockon_time, padding, min_distance, max_distance, fov=90, znear=0.1, zfar }` with `adjust(bounds, viewport: (u32,u32))`, `logic(dt)` | APP ZoomCamera section. `toa = tan(fov*0.5*0.017453292)*2.0` uses the f64 literal `0.017453292` (pi.h:33) stored to f32: 1.99999988, not 2.0 (corrected tick 4: verification-tick3-scene.md row 4). Dir `file_area` constant stored f32 is 50.26548386 (row 5). `adjust` needs `display.width/height` -> take a `Viewport` argument, do not read a global |
| `GourceSettings` | `struct GourceSettings { ... one typed field per SET sec. 4/5 key ... }`, plus `DisplaySettings` | `import_gource_settings(conf_section)` resets to defaults first (SET T2.1 step 3) |
| `Gource` | `struct Gource { settings, commitlog: Option<Box<dyn CommitLog>>, logmill, commit_queue: VecDeque<Commit>, users: BTreeMap<String, UserId>, files: BTreeMap<String, FileId>, tree: DirTree, tag_users: BTreeMap<i32,..>, user_tree/dir_tree: QuadTree, camera, runtime: f32, currtime: i64, subseconds: f32, framecount, ... }` | APP T2.6 table says which members are SIM |
| Process globals | `gStringHashSeed`, `gGourceRemovedFiles`, `gGourceDirMap`, `gGourceForceGravity` | Make them fields of a `SimCtx { hash_seed: i32, removed_files: Vec<FileId>, rng: Prng }` passed `&mut`. No `static mut` (needed also for multi-section reset, SET T2.1) |

### 2.4 Where `BTreeMap` replaces `std::map`, and where `Vec` order must stay

| Container (C++) | Rust | Why / source |
|---|---|---|
| `users: map<string,RUser*>` | `BTreeMap<String, UserId>` | sorted-by-name iteration drives `interactUsers`, `updateUsers`, rand() order (SCN sec. 0 table). Byte-wise `<` equals `String` `Ord` (UTF-8 byte order); do not use a locale collation |
| `files: map<string,RFile*>` | `BTreeMap<String, FileId>` | sorted by fullpath |
| `gGourceDirMap: map<string,RDirNode*>` | `BTreeMap<String, DirId>` | sorted by abspath with trailing '/' |
| `tagusermap` | `BTreeMap<i32, UserId>` | APP T2.6 |
| `FileKey::keymap: map<string,...>` | `BTreeMap<String, ..>` | ordered by extension (APP key.cpp) |
| `ConfFile: map<name, list<ConfSection*>>`, `ConfSection: map<key, list<ConfEntry*>>` | `BTreeMap<String, Vec<ConfSection>>`, `BTreeMap<String, Vec<String>>` | `--save-config` output order is sorted by section/key, repeats in insertion order (SET sec. 2) |
| `RDirNode::children/files` (`std::list`) | `Vec` (insertion order, `remove`, never `swap_remove`) | SCN sec. 0 |
| `gGourceRemovedFiles` (`vector`) | `Vec<FileId>` | expiry order; touch-after-expire erases the entry |
| `RUser::actions/activeActions` (`list`) | `VecDeque` | FIFO |
| `commitqueue` | `VecDeque<Commit>` | refill rule `queue.empty() || (back.timestamp <= currtime && size < 100)` (LOG sec. 1 item 8). **Commits are never sorted** (corrected tick 4; verification-tick3-scene.md sec. 2; gource.cpp:1131-1154, 1744-1774): consumed in log order via a one-commit pushback buffer (`lastCommit`, commitlog.cpp:128,212-219); pop from the front while `front.timestamp <= currtime`, so an out-of-order commit is processed at its log position; without `--no-time-travel`, `currtime = commit.timestamp` when `lasttime > commit.timestamp` (:1765-1767). Do not add a sort or a BinaryHeap |
| quadtree leaf items | `Vec<Item>` per node, visit TL,TR,BL,BR, items in insertion order, tree rebuilt every tick | COR T2 (c); an O(n^2) or hash-set substitute is not equivalent |
| `getFilesRecursive` | build by prepending (later dirs first) | SCN 3.2; reproduce literally or insert-at-0 then reverse once |

HashMap must not be used anywhere in sim state (random iteration order in Rust).

## 3. Threading model

### 3.1 LogMill reader (APP logmill section; LOG sec. 1)

C++: one SDL thread started in the ctor; state is a plain int (STARTUP/FETCHING/SUCCESS/FAILURE) read by main without atomics; `getLog()` joins; `abort()` only joins. The main thread polls `isFinished()` each frame; frames run (and `runtime`/`framecount` advance) while `commitlog==0` (TIM sec. 1).

Rust:
- `std::thread::spawn` in `LogMill::new`, returning the boxed `CommitLog` (or error `String`) via `JoinHandle<Result<Box<dyn CommitLog + Send>, LogError>>`; `AtomicU8` state for `is_finished()`; `getLog()` = `join()`. Use `Result<_, LogError>` with the exact message strings (APP sec. 13, LOG sec. 1 item 6).
- After handoff the Gource owns the log and reads it on the main thread (as C++ does: `readLog` is called from `Gource::logic`); the commit queue is therefore a plain `VecDeque` on the main thread, not a channel. Only the *detection and initial seek-to-start-timestamp* run on the worker, matching `RLogMill::run` (APP logmill section).
- stdin: non-blocking read semantics must be kept ("no line yet" is not EOF, final unterminated line dropped, LOG sec. 4). Do not use blocking `read_line` on the main thread; either set `O_NONBLOCK` via `libc` / `rustix`, or run a dedicated stdin pump thread feeding an `mpsc::Receiver<Vec<u8>>` polled with `try_recv`. Either approach is fine (decided 2026-10-31: no native Windows build, so `PeekNamedPipe` portability is not a factor); preserve the "unterminated last line dropped" rule explicitly.
- SIGINT unblocked in the worker on POSIX (APP logmill): do not install a handler in the worker; leave default.
- Parity wrinkle: because load time shifts `framecount` phase and `runtime` offset (TIM sec. 3 Determinism row), Rust blocks (`join`) until the log is ready before frame 0 in fixed-dt/export mode. This is intentional deviation D2 from C++, not parity (decided 2026-10-31; see sec. 7.2 and sec. 10). The worker also runs `fetchLog` format detection and the `--start-date` scan (consumes commits until `timestamp >= start_timestamp`, then `bufferCommit`s it; logmill.cpp:86-97); a `SeekLogException` becomes `unable to read log file`.

### 3.2 PPM writer (OUT sec. 1 Threading row)

C++: one thread `frame_exporter`, mutex + cond, two pixel buffers + `pixels_out`; main thread does `glReadPixels` into the idle buffer, swaps, signals; the worker flips rows and writes. If the worker is still writing, main blocks (no drop, queue depth 1).

Rust: `std::sync::mpsc::sync_channel::<Vec<u8>>(1)` is a faithful equivalent (send blocks when one frame is already queued and the worker is busy). Better: two pre-allocated buffers recycled through a second return channel (`sync_channel(1)` back) to avoid per-frame allocation. Worker: take frame, flip rows (GL bottom-up -> top-down), write `P6\n{w} {h} 255\n` header then bytes through `BufWriter<Stdout|File>`; header on EVERY frame; no per-frame flush; flush on drop/exit. Handle `BrokenPipe` by terminating cleanly (C++ ignores errors, OUT sec. 1 Rust notes). Stdout lock held by the worker only; no other code may print to stdout while `-o -` is active (help/info paths exit before this, SET sec. 1).

The `crossbeam-channel` crate is optional (DEP threads row); std suffices here.

## 4. Renderer boundary

### 4.1 Trait the sim never depends on

Sim exposes plain read-only data; `gource-render` turns it into batches; the backend draws batches. Sketch:

```rust
pub trait Renderer {
    fn begin_frame(&mut self, clear: Vec4, viewport: (u32, u32));
    fn set_camera(&mut self, view: Mat4, proj: Mat4);               // 3D passes
    fn draw_batch(&mut self, b: &Batch);                              // Batch { pipeline, texture, blend, uniforms, vertices }
    fn draw_text(&mut self, buf: &TextBuf);                           // glyph quads, atlas owned by backend
    fn text_metrics(&mut self, font: FontId, s: &str) -> Vec2;        // widths for layout (label placement, key, slider)
    fn read_back_rgb(&mut self, out: &mut [u8]);                      // for PPM
    fn present(&mut self);
}
pub enum Pipeline { Textured, Shadow { strength: f32 }, Text { strength: f32, texel: f32 }, Bloom, Solid }
```

Sim-to-render contract: `fn build_frame(&Gource, &Settings, &RenderState) -> FrameData`. `RenderState` (frustum flags, projected positions, hover, VBOs) is owned by render, keyed by `DirId/FileId/UserId` (APP T2.7: calcEdges, VBO fills, calcScreenPos do not touch sim state). `gluProject`/`gluUnProject`/`gluPerspective`/`gluLookAt` become `glam` code in `gource-render` (DEP risk 3), because label placement and mouse picking depend on them (`file.cpp:274`, `user.cpp:376`, `dirnode.cpp:949`).

Mouse picking (`mousetrace`) is the one render-to-sim feedback path (APP T2.7: it selects user/file, locks the camera, deselects on fade). Keep it in `gource` (app) as an input step that calls `Gource::select_user/select_file` after a CPU unproject; the sim crate sees only selection commands. Skip it in headless/dump mode.

### 4.2 Stack decision (decided 2026-10-30)

Chosen stack: `winit` 0.30.x + `glutin` 0.32.x + `glutin-winit` 0.5.x + `glow` 0.18 + `glam`, versions pinned exactly, targeting OpenGL 3.2 core, `#version 150`. The window and GL context stay behind the `Renderer`/`GlContext` traits (sec. 4.1) so the sim and render crates never see winit or glutin types. The user chose this over the recommended `sdl2` 0.38 + `glow` (DEP T2.2 recommendation A), which is now a rejected alternative, as is wgpu (WGSL rewrite and no GLSL 1.10 ingest, DEP T2.2 option C). Cost accepted: more hand-written input-behaviour tests than SDL's 1:1 semantics. macOS caps at GL 3.2 core (sec. 8).

### 4.3 Render pass checklist (order from OUT sec. 4, `Gource::draw` gource.cpp:2401-2780)

Reference mode is shaders (`--ffp` need not be ported; OUT sec. 4 intro; COR sec. 0 notes `--ffp` is first-class in C++, so decided 2026-10-31 (D4): `--ffp` is accepted and ignored, rendering always uses the shader path; the `P` key toggle (APP lines 294, 554) becomes a no-op).

- [ ] 0 clear + optional background image; blend SRC_ALPHA, ONE_MINUS_SRC_ALPHA (same factors for alpha)
- [ ] loading screen when no commitlog ("Reading Log"/"Aborting" + dots from `int(runtime*3)%4`)
- [ ] 1 camera: perspective fov 90, aspect w/h, near 0.1, far `zfar`; frustum cull; CPU project; VBO build
- [ ] 2 edges: screen-space ortho, `beam.png`, shadow pass (strength 0.5, +2,+2 px) then normal pass; quad per spline segment, half-width 2.5 px
- [ ] 3 file shadows (strength 0.5, +2,+2 world)
- [ ] 4 actions (beam quads, alpha ramp, `beam.png`)
- [ ] 5 files
- [ ] 6 user shadows ((2,2)*user_scale)
- [ ] 7 users (per-texture runs)
- [ ] 8 bloom: blend ONE,ONE, bloom shader, ignore bloom.tga; fix reversed smoothstep per OUT sec. 8 item 2 only if visually required (it exists in the C++ shader)
- [ ] 9 text: dir names, user names (text shader strength 0.7, texel size 1/min(512,GL_MAX_TEXTURE_SIZE)); selected names with CPU shadow
- [ ] 10 debug overlays (optional)
- [ ] 11 2D UI: logo, splash (15 s), date, title, captions, file key, slider, hover textbox, message; screenshot taken here (includes UI, excludes cursor)
- [ ] 12 outside draw: PPM dump (after UI, before cursor), then cursor
- [ ] multi-repo transition: capture last frame to texture, blend with alpha `transition_interval` decreasing by wall dt (OUT sec. 4 notes; SET T2.1 step 1)

### 4.4 Shader rewrite checklist (GLSL 1.10 compat -> `#version 150`; DEP T2.2; OUT sec. 5)

Files: `data/shaders/{shadow.vert, text.vert, shadow.frag, text.frag, bloom.vert, bloom.frag}`.

- [ ] Add `#version 150` as line 1 (the existing preprocessor, COR sec. 6, is dropped)
- [ ] vertex: `gl_Vertex`/`ftransform()` -> `in vec3 position; uniform mat4 mvp;` (CPU keeps the matrix stack in glam)
- [ ] vertex: `gl_MultiTexCoord0` -> `in vec4 texcoord` (bloom.vert reads `.yzw` as extra data, so keep full vec4)
- [ ] vertex: `gl_Color` -> `in vec4 color`; `gl_FrontColor`/`gl_TexCoord[0]` -> `out vec4 v_color`, `out vec4 v_texcoord`
- [ ] bloom.vert/frag: `varying vec3 pos` -> matching `out`/`in`
- [ ] fragment: `varying` -> `in`; `texture2D` -> `texture`; `gl_FragColor` -> `out vec4 frag_color` (bind with `layout`-free `glBindFragDataLocation` or only one output)
- [ ] uniforms preserved by name: `tex`, `shadow_strength`, `texel_size` (OUT T2.a for plumbing)
- [ ] immediate mode and `GL_QUADS` -> triangle index buffers (0,1,2, 0,2,3); wide lines (`glLineWidth`) -> quads
- [ ] no depth buffer needed (OUT sec. 8 item 7); `GL_CORE` VAO required, create one global VAO
- [ ] mipmaps via `glGenerateMipmap` (replaces `gluBuild2DMipmaps`), trilinear for file/user textures; `beam.png` wrap REPEAT (OUT sec. 8 item 6)
- [ ] font atlas: A8 `GL_R8` (core has no `GL_ALPHA`/`GL_LUMINANCE`) with a swizzle or sample `.r`

## 5. Settings and CLI model

### 5.1 Recommendation: hand-rolled parser, not clap

Reasons:
- `parseArgs` does not behave like a normal CLI: all leading `-` stripped (`-x`, `--x`, `---x` identical), a lone `-` is positional, `--` is "unknown option -", no `--opt=value`, a value-taking option consumes the next token even if it starts with `-`, and a `-WxH` token (regex `^([0-9.]+)x([0-9.]+)(!)?$`, both ints >0) is a magic viewport (SET sec. 3 items 1-6). clap would reject or reinterpret most of these.
- Help output must match the Gource goldens (`golden/help-short.txt`, `help-long.txt`) byte for byte except the recorded D5 help-text diff (user-friction default 1.0; dir-name-position range starts at 0.1), which will live as a unified diff under `golden/expected-diffs/` (does not exist yet); the goldens themselves are never edited. Exit 0 to stdout; clap-generated help cannot do this. Embed the corrected text with `include_str!`; no Windows `cfg` branches are needed (no native Windows build).
- Errors are `<program>: <msg>\nTry '<program> --help' for more information.\n\n` on stderr, exit 1 (the C++ prints `gource`; Burl prints `burl`, and the golden harness normalises the program name only at six anchored phrases, see `crates/burl/tests/golden.rs`) (SET sec. 1 error format). Messages come from ConfFile and import validation, not from the arg library.
- Behaviours like "CLI scalars applied to every `[gource]` section" need direct access to the ConfFile structure (below), which a derive-based parser would obscure.

clap may still be used in tests only to cross-check the option index; do not ship it. The option table (`arg_types`, `arg_aliases`, section routing) comes from SET sec. 8; make it one `static` table `(name, type: Bool|Int|Float|String|Multi, section: Display|Gource|CommandLine)` and generate parse/help-index tests from it.

### 5.2 ConfFile semantics to reproduce

| Behaviour | Spec | Source |
|---|---|---|
| Model | `BTreeMap<String, Vec<Section>>`; `Section = BTreeMap<String, Vec<String>>`; values stay raw strings; typed conversion only at import | SET T2.1 storage, sec. 11 |
| Grammar | line-based; `#` comment only at column 0; empty lines skipped; `\r`-only line fails; section regex `^\s*\[([^\]]+)\]\s*$`; entry regex `^\s*([^=\s]+)\s*=\s*([^\s].*)?$`; trailing `" \t\f\v\n\r"` trimmed; no quoting; `key=value` before any header goes to a `""` section never imported; error `<file>, line <n>: could not parse line` | SET sec. 2.1 |
| Unknown keys/sections | silently ignored by import | SET sec. 2.1 item 6, T2.1 |
| Reads | single-valued key = FIRST entry of that key; multi-value keys read all | SET sec. 2.1 item 5 |
| `--save-config` | write sections sorted by name (repeats in insertion order), `[name]\nkey=value\n...\n\n`; `failed to write config to <file>` | SET sec. 2 |

### 5.3 parseArgs-into-ConfFile semantics

Two passes, exactly the order of SET sec. 1 steps 3-11 / T2.1 "Load order in main":

1. `parse_args(argv, &mut conf, Some(&mut files))` into an empty conf. Command-line-section options (`-h`, `-H`, `--load-config`, `--save-config`, `--log-command`, `--*-command`, `--output-custom-log`, `--log-level`) run immediately, in both passes, never stored (SET sec. 3.2); so `--log-command git` prints and exits 0 during pass 1.
2. If no `--load-config` and positional files exist: the first positional ending `.conf` (len>5), `.cfg` or `.ini` (len>4) that loads without error becomes `load_config` and is removed from `files`.
3. If `load_config`: `conf.clear(); conf.load(file); parse_args(argv, &mut conf, None)` (CLI over file, positionals dropped).
4. If positionals remain: the LAST is `path`; `set_entry("path", ..)` on every `[gource]` section, or create one.
5. `import_display_settings` (first `[display]` only), `import_gource_settings` (first `[gource]` only, defaults reset first, see item 8). `--output-custom-log` exits 0 only if the imported `path` is non-empty (main.cpp:119). Then `--save-config` -> save, exit 0; `--output-custom-log` non-empty -> write log, exit 0.
6. Per-option application: for non-command options, apply to EVERY section of the target name: Multi types `add_entry` (config entries then CLI entries); others `set_entry` (replace all entries of that key in that section). Bool options store `"true"`. Magic `-WxH` writes `[display] viewport=<token as typed>`.
7. **INTENTIONAL DEVIATION D1 (not C++ parity; decided 2026-10-31; corrected tick 4)**: C++ validates later `[gource]` sections lazily in `Shell::get_next` (SET T2.1 gotcha a), so earlier repos play before a bad later section terminates the process via uncaught exception. Proposed Rust behaviour: validate ALL `[gource]` sections up front in step 5 and exit 1 with the same message before anything plays. Only failure timing changes. There is no flag: the lazy C++ behaviour is not reproduced.
8. **Per-repo settings reset set (C++ behaviour to reproduce; misc-traces.md sec. 3, gource_settings.cpp:368-520, 613)**: each `[gource]` section is imported into a FRESH default `GourceSettings` (C++ calls `setGourceDefaults()` at the start of `importGourceSettings`, and again in the `Gource` dtor, gource.cpp:223). The reset covers every per-repo field (path ".", hide_*, start/stop dates and positions, stop_at_time -1, stop_at_end, dont_stop, loop, days_per_second 0.1, time_scale 1, auto_skip 3, camera zoom 50/100/10000, bloom, colours, fonts, caption_*, filters cleared and freed, `follow_users`/`highlight_users`/`user_image_map` cleared, user_friction 1, ...) AND the global `gStringHashSeed = 31`, so `SimCtx.hash_seed` is re-initialised per repo (and overwritten by the recolour `rand()` path, gource.cpp:1555). NOT reset: `repo_count`, `shutdown`, `load_config`, `save_config`, `output_custom_filename`, `file_graphic` texture, `log_level`, all display-level settings (imported once), the ConfFile. So `Shell` holds `(DisplaySettings, ConfFile, repo_count)` once and builds `GourceSettings` per section; the not-reset fields live outside `GourceSettings`. Per-repo fixups after import (gource_shell.cpp:221-242): exporter present and not (`dont_stop || loop || path=="-"`) => `stop_at_end=true`; `repo_count>1 && stop_at_time<=0 && stop_position<=0` => `stop_at_time=60.0`; wrap to the first repo only when `repo_count>1 && exporter==0`. (timing.md sec. 2 last paragraph says re-import resets "only options present"; misc-traces.md and the source say defaults are reset first, so the latter is right.)

Exit codes: `exit(1)` for `SDLAppQuit` errors, `exit(0)` for help/info/save/custom-log (SET sec. 1; golden `exit-codes.txt`). Version string `0.57` (`GOURCE_VERSION`). **There is no `--version` option** (corrected tick 4): `gource --version` is an unknown option, stderr `gource: unknown option version` plus the Try line, exit 1 (`golden/version.stderr.txt`, `exit-codes.txt` rc=1). `Gource v0.57` is only the first line of `-h`/`-H` output (gource_settings.cpp:50); DEP sec. 1 wording "printed by `--version`" is wrong.

Typed settings: `import_*` validators produce the exact messages catalogued in SET sec. 4-6 and APP sec. 13; test each against `golden/exit-codes.txt` plus additional negative cases captured from the C++ binary.

## 6. Regex strategy

| Use | Engine | Why / source |
|---|---|---|
| `--file-filter`, `--file-show-filter`, `--user-filter`, `--user-show-filter` (public API, arbitrary PCRE2) | `pcre2` crate, no UTF/UCP/caseless flags (flags=0), byte subjects | exact parity incl. look-around, backrefs; `\w \d \s` ASCII (LOG sec. 3; DEP regex row recommendation). Invalid pattern -> `invalid file-filter regular expression` style message (GS:1525-1531) |
| `--git-branch` validation `^(?!-)[/\w.,;_=+{}\[\]-]+$` (negative lookahead) | `pcre2` (or equivalent explicit check: first byte != `-` and rest matches class) | gource_settings.cpp:743 (LOG sec. 3) |
| Internal parser patterns (git, svn, hg, bzr, cvs, apache, custom, parseDateTime, conffile) | `regex::bytes::Regex` with `(?-u)` | no look-around used (LOG sec. 3 "Features in log parsers"); BOM `\xEF\xBB\xBF` works in bytes mode |
| Hot loops (git line parsing) | hand-written byte scanning where trivial | optional optimisation; must keep identical results |

Build note: `pcre2` needs `pcre2-sys`, which can vendor-build with a C compiler; document that as the one native build dependency besides FreeType (SDL2 is no longer a dependency, sec. 4.2) (DEP regex row).

`gource_util::Regex` wrapper semantics (LOG sec. 3, regex.cpp:149-208) that every call site relies on:

- [ ] unanchored search (`is_match`/`captures` search from offset 0, not full match)
- [ ] byte mode: `&[u8]` subjects; no Unicode classes; `.` excludes `\n`; `^`/`$` single-line, but PCRE2 `$` also matches before a final `\n` while Rust `regex` `$` does not (corrected tick 4: pcre2_compile flags are 0, regex.cpp:33-39; keep the `pcre2` engine for user filters, and for internal patterns confirm no subject can end in `\n`)
- [ ] `match(str) -> Option<Vec<Vec<u8>>>` returns groups 1..=N where N is the highest participating group: trailing non-participating groups are dropped; middle non-participating groups become empty; callers use `results.len()` (custom colour `>=5`, apache `!=4`/`!=8`/`<5`, parseDateTime)
- [ ] empty subject never matches, even for `^$` (`matchOffset` returns -1 when `offset >= str.size()`), so user filters never match an empty username
- [ ] `replace`/`replace_all` with `$N` references (used outside log code)
- [ ] construction fails with `RegexCompilationException` unless `test` mode; user filters use test mode and surface the settings error
- [ ] one wrapper type implemented over both engines (`enum Engine { Pcre2(..), Bytes(..) }`) so callers do not care

Differential test: the 4 filter options and the git-branch check against the C++ binary (DEP regex row).

## 7. PRNG and timing

### 7.1 PRNG (SCN sec. 9; APP T2.8; DEP T2.1)

`rand()` is used at four sites only: dir-dir overlap (dirnode.cpp:625), user-user and user-file overlap (user.cpp:106, 128) as `normalise(vec2(rand()%100-50, rand()%100-50))`, and `rand()%10000+1` for recolour (gource.cpp:1555); `srand(time(0))` plus `(rand()%1000)/1000.0f` only for `--start-position random`. Call order is fixed by users-in-map-order then dirs recursively, so the Rust sim must call the PRNG at the same points in the same order.

Plan (consistent with LEDGER key findings and parity-test-spec.md sec. 2.4; corrected tick 4: the earlier text made glibc the default):
- `trait SimRng { fn next_i31(&mut self) -> i32 /* 0..=2^31-1 like libc rand() */; }` held in `SimCtx`, with a call counter.
- **Default (production)**: Rust's own documented, platform-independent PRNG (`Xorshift` in the parity spec) with a fixed default seed, so runs are reproducible; `--start-position random` is the only wall-clock seed.
- **Compatibility modes** (decided 2026-10-31, D3: glibc and macOS modes exist for parity tests), selected by env `GOURCE_RS_PRNG=xorshift|glibc|park|script` (one mechanism, as in parity-test-spec.md sec. 2.4; recorded in the `--debug-dump-state` header; no documented public CLI behaviour changes): `Glibc` (TYPE_3, seed 1, `srand(0)` maps to 1; about 15 lines), `Park` (macOS Park-Miller), `Script` (values from `--rand-script FILE`, cyclic; the primary mode of the parity matrix because both binaries can read it).
- The glibc first outputs `1804289383, 846930886, 1681692777, 1714636915` come from a scratch Python reimplementation (DEP T2.1), not from a libc run, and the parity spec (sec. 2.4, 6) marks the glibc and macOS algorithms `unverified`. Confirm against a real libc in CI before relying on them (the macOS mode against a recorded macOS libc sequence). Bit-exact C++ parity is per platform and only in the matching mode; cross-platform comparison uses `Script` or the tolerance tier (TST sec. 6 (2), parity spec 2.6).
- Pin the argument order of `vec2(rand()%100-50, rand()%100-50)`: C++ leaves it unspecified and clang (the golden compiler, BLD) evaluates x first (SCN Tick 3 corrections; verification-tick3-scene.md row 9). Draw x then y in every Rust mode and record the reference compiler.
- `--start-position random` seeds from `time(0)` (non-reproducible by design).

### 7.2 Timing (TIM is authoritative)

| Item | Rust implementation | Source |
|---|---|---|
| Interactive loop | accumulate wall ms; skip until `buffer >= 16`; `delta = buffer` (whole accumulated amount, no remainder carried); `t = total/1000 f32`, `dt = delta/1000 f32`; then poll events, `update`, swap | TIM sec. 1 |
| Min delta | **16 ms** (GourceShell override of SDLApp's 8) | TIM sec. 1, COR sec. 0 |
| `GourceShell::update` | if no current Gource or it `is_finished`: call `get_next()` and return, with no update/draw/dump that iteration (gource_shell.cpp:285-288); the swap still runs | TIM sec. 1 |
| `Gource::update` | `scaled_dt = min(dt, 1/60)`; with exporter `scaled_dt = max_tick_rate`; `*= time_scale`; `runtime += scaled_dt` unless paused; stop-at-time check; `logic`; `draw`; dump; cursor with REAL dt; `framecount += 1` | TIM sec. 1, 3 |
| Fixed-dt (exporter) path | `gource_fps = F; skip=0; while gource_fps < 60 { gource_fps += F; skip += 1 }`; `max_tick_rate = 1/gource_fps`; dump when `framecount % (skip+1) == 0` and the log is loaded and not shutting down. F=60 -> 60 Hz skip 0; F=30 -> 60 Hz skip 1; F=25 -> 75 Hz skip 2 | TIM sec. 3, OUT sec. 1 |
| `--headless`-style fixed mode | The Rust sim must expose `step(&mut self, dt: f32)` where interactive and exporter paths feed the same function; the exporter path just supplies `max_tick_rate`. A dump-state run uses that fixed path without a window (same as exporter) | TIM sec. 3, TST sec. 6 (2) |
| Log-load phase | In C++, frames before `commitlog` exists still advance `runtime` and `framecount`, so the dump phase depends on load time (TIM sec. 3). **INTENTIONAL DEVIATION D2 (not C++ parity; decided 2026-10-31; corrected tick 4)**: in fixed-dt/export mode Rust blocks (`join`) until the log is ready before frame 0, with `framecount` phase 0 and `runtime` 0 at the first loaded frame. C++ output in that phase is load-time dependent, so no byte-exact C++ comparison exists for it anyway. An old-behaviour flag is OPTIONAL and was not specifically requested: add a hidden flag (name chosen then) only if the parity comparator needs C++ log-load timing. Interactive mode keeps C++ behaviour (no deviation) | TIM sec. 3 |
| Seconds to days | `time_inc` is `f32`: `let time_inc: f32 = (dt as f64 * 86400.0 * days_per_second as f64) as f32;` then `seconds = time_inc as i32` (truncate), `subseconds(f32) += time_inc - seconds as f32`; if `subseconds >= 1.0 { currtime += subseconds as i64; subseconds -= (subseconds as i32) as f32 }`; then `currtime += seconds` (corrected tick 4: app-core.md 242/356 said f64; gource.cpp:1723-1733 declares `float`; verification-tick3-scene.md row 2) | SCN sec. 0 item 3, APP table row 12 |
| Stop conditions | `stop_at_end` forced when exporting (unless `dont_stop`/`loop`/path `-`); trailing fade-out frames still exported until `users.empty() && stop_position_reached`; multi-repo default `stop_at_time=60` only when `stop_at_time<=0 && stop_position<=0` (gource_shell.cpp:228-233) | TIM sec. 4, OUT sec. 1 |
| Wall-clock reads that affect sim | start/stop date parsing uses local TZ `mktime`; `srand(time(0))`; non-UTC parsers use `mktime`/`timegm` (TZ dependent) | TIM sec. 5 |
| Time library | `chrono` (Local + explicit offset) or direct libc `mktime`/`timegm` calls via `libc`; recommend `libc` for exact TZ semantics, `chrono` only for tests | DEP dates row, LOG sec. 8 |

`SDL_GetTicks` is replaced by `Instant::now()` in the interactive loop only; the sim never reads the clock.

## 8. Platform notes

| Platform | Notes | Source |
|---|---|---|
| macOS | OpenGL is capped at 3.2 core and deprecated: request profile core 3.2, forward-compatible, `#version 150`, no `GL_QUADS`/`GL_ALPHA`/`glLineWidth>1`. Default window height 640 (768 elsewhere). Frameless toggle disabled at runtime. `--high-dpi`: C++ disables high-DPI on non-Windows when a viewport is specified without `--high-dpi`. NFC-normalise avatar file names (`user-image-dir`). `rand()` differs from glibc (sec. 7.1). Homebrew libs here are x86_64 (BLD) so run the C++ reference via Rosetta when capturing goldens | DEP T2.2; SET sec. 9; COR sec. 16; BLD |
| Windows | No native Windows build in the first release (decided 2026-10-30). Windows users run a Docker image (Linux userland) as headless frame export only | decisions |
| Linux | Reference CI target. No GL needed for parser/sim gates. Render gate: Xvfb + Mesa llvmpipe (`LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe`, request a core 3.2 context; glutin config selection, GLX or EGL, under Xvfb and Wayland). Wayland: C++ tolerates `GLEW_ERROR_NO_GLX_DISPLAY`; with glow no GLEW exists so skip | TST sec. 4, 6 (3); COR sec. 16 |
| All | Resource lookup order and `SDLAPP_RESOURCE_DIR` define (COR sec. 3); fonts `FreeSans.ttf` embedded via `include_bytes!` or installed; log commands run through `system()` with a temp file (git/hg/bzr/svn/cvs flags verbatim, DEP risk 6; quoting per LOG sec. 2 Windows row) | COR, LOG |

Font choice: use `freetype-rs` for hinted glyphs to keep label metrics close (DEP font row, risk 2); `fontdue` is the pure-Rust fallback with unhinted drift.

## 9. Phased implementation order and parity gates

Gate tools exist in the repo: `docs/rust-rewrite/discovery/golden/` (help, version, exit codes, custom-log for 5 fixtures) and `scripts/` to rebuild/recapture with clang++ (BLD).

| Phase | Deliverable | Parity gate | Source |
|---|---|---|---|
| 0 | workspace skeleton, CI, `gource-util` (strhash, utf8, bounds, math, regex wrapper, prng, timeparse) | `cargo test`: port `src/test/{datetime,regex}` cases; stringHash/colourHash vectors; glibc-mode PRNG first outputs `1804289383, 846930886, 1681692777, 1714636915` (once confirmed against a real libc, sec. 7.1) and the macOS compatibility mode against a recorded macOS libc sequence plus a determinism test of the default Rust PRNG; `replace_invalid` cases | TST sec. 1; DEP T2.1; COR T2 (d)(e) |
| 1 | `gource-conf`: ConfFile, parse_args, settings import, help text, error strings, `--save-config`, `--log-command` | byte-exact diff of `-h`, `-H`, `--version`, exit codes against the oracle goldens `golden/help-short.txt`, `help-long.txt`, `version.stderr.txt`, modulo the recorded D5 help diff (`golden/expected-diffs/`, does not exist yet) (there is no `--version` option: `gource --version` exits 1 with `unknown option version` on stderr; version.txt is empty), `exit-codes.txt`; round-trip `--save-config` vs C++ on a matrix of args (multi-section included) | SET sec. 7, 3; BLD |
| 2 | `gource-formats`: SeekLog/StreamLog, nine parsers, logmill detect order, filters, `--output-custom-log` | `burl --output-custom-log - tests/logs/*.log` equals `golden/custom-log/*` exactly; plus captured git/svn/hg/bzr samples through `--log-format`; edge inputs (empty, CRLF, BOM, out-of-order, `|` in names, stdin unterminated line) | TST sec. 6 (1); LOG sec. 9 |
| 3 | `gource-sim` headless: DirTree, File, User, Action, QuadTree, Gource::logic, fixed-dt `step`, `--debug-dump-state` (added to the C++ fork at `reference/gource-cpp-dump/` (does not exist yet; it must first reproduce the oracle's help and custom-log goldens, and the untouched original build stays the oracle) and to Rust, JSONL as in TST sec. 6 (2)) | Exact tier: topology, add/remove/idle-expire frames, user set, colours. Position tier: exact when the glibc RNG is selected and sim built with matching f32 op order, else 1e-3 relative. Run `file-removal.log` etc. with `--hash-seed 31 --disable-auto-rotate --stop-at-time 10` | TST sec. 6 (2); SCN sec. 10 |
| 4 | Camera, auto-rotate, selection, captions timeline, multi-section shell (still headless) | camera pos/zoom fields in the dump; multi-section run of `tests/test.conf` matches section-by-section counts | APP; SET T2.1 |
| 5 | `gource-gl` renderer: window, shaders (sec. 4.4), passes (sec. 4.3), font atlas, textures, screenshot, PPM writer thread | `-o` stream is valid P6 with the per-frame header and correct frame count (`ffmpeg` decode); frame cadence from sec. 7.2 (frames dumped per runtime second for F=25/30/60); SSIM >= ~0.9 vs C++ reference frames under Xvfb/llvmpipe on every 30th frame; structure checks with bloom and shadows off | TST sec. 6 (3); OUT sec. 1, 8 |
| 6 | UI overlays (key, slider, textbox, logo, splash), input bindings, interactive loop with 16 ms accumulation, HiDPI | manual checklist against APP T2.2 key/mouse table; no regression in earlier gates | APP T2.2 |
| 7 | packaging, resource lookup, cross-platform CI | `cargo build` on Linux and macOS natively; Windows via the Docker headless-export image; Linux render gate in CI | DEP sec. 5, 7 |

Rule: a phase may not start before the previous gate is green; the sim gate (phase 3) is the rewrite's main correctness bar (visual gate is intentionally loose).

Sabotage check for each gate test (user convention): temporarily break the code under test (e.g. flip the `children` iteration order, return `from_utf8_lossy` output, drop the empty-subject rule) and confirm the corresponding gate fails before trusting it.

## 10. Decisions taken and known quirks

All rows decided 2026-10-30/31 (D1 to D5 as numbered in the LEDGER Decisions).

| Item | Handling | Source |
|---|---|---|
| `--log-format cvs-exp` unmatched by logmill (`"cvs"` only) in C++ | fix (D5): explicit `cvs-exp` selects the CVS-EXP parser; auto-detect still works | LOG sec. 1 item 7 |
| `--stop-on-idle` no-op | accepted option, documented as having no effect (decided 2026-10-31, D5) | OUT sec. 1 |
| help text says user-friction 0.67 but code default 1.0 | fix (D5): corrected help text says 1.0, as implemented; the Gource golden is not edited, the diff goes under `golden/expected-diffs/` | LEDGER key findings |
| help text gives `--dir-name-position` range starting at 0.0, code starts at 0.1 | fix (D5): corrected help text starts the range at 0.1, recorded in the same expected diff | settings-cli.md |
| Later-section validation lazily terminates | **DEVIATION D1**, decided: validate all sections up front (sec. 5.3 item 7); no flag | SET T2.1 gotcha |
| Log load in export mode | **DEVIATION D2**, decided: block on log load before frame 0 (sec. 7.2); old-behaviour flag optional, not requested | TIM sec. 3 |
| `--ffp` (fixed-function path) | accepted and ignored (D4); shader path only; `P` key toggle is a no-op | COR sec. 0 |
| `RDirNode` ctor shadowed `parent` | no ctor parent parameter in Rust (sec. 2.2) | SCN 3.1 |
| Uninitialised C++ fields | glm vectors are zero by definition (GLM_FORCE_CTOR_INIT); only `QuadItem::node_count` is truly uninit, use 0 (corrected tick 4) | verification-tick3-scene.md row 1 |
| PI constant | `PI` is `3.14159265` f64; use that constant for sim, not `std::f64::consts::PI` | SCN sec. 1 |
| Gaps not resolved by discovery | RUser/RFile/RDirNode force formulas and ZoomCamera::adjust still await independent re-derivation; (corrected tick 4: the commit-sorting question is closed, commits are never sorted, sec. 2.4). Force formulas and `ZoomCamera::adjust` were independently re-derived in tick 3 and found consistent (verification-tick3-scene.md); the phase 3 gate remains the final check | LEDGER, verification-tick3-scene.md |

## Verification (tick 4)

Cross-checked against Makefile.am, source, and timing.md, scene-model.md (incl. Tick 3 corrections), app-core.md T2, log-formats.md, settings-cli.md, misc-traces.md, verification-tick3-scene.md, parity-test-spec.md.

Corrections made (each marked "corrected tick 4" in place):
- `--version` does not exist (exit 1, unknown option); `Gource v0.57` is the first line of help output.
- `time_inc` is f32 (was ambiguous; app-core.md 242/356 disagree with source).
- Commits are never sorted; sec. 2.4 and the sec. 10 gap row updated.
- PRNG default is a documented Rust PRNG with optional glibc/Park/Script modes (was glibc default); glibc first outputs flagged as verified only by a scratch Python script.
- Per-repo reset set added (sec. 5.3 item 8), including `gStringHashSeed=31` and the not-reset list; stop_at_time=60 condition and `stop_at_end` fixup stated precisely.
- Two intentional deviations now labelled D1 (validate all sections up front) and D2 (block on log load in export mode) not parity; both decided 2026-10-31 (D1 has no flag, a D2 flag is optional).
- Makefile.am coverage: every built .cpp has a row; tinyxml is conditional (USE_BUNDLED_TINYXML); utf8/ftgl are header-only; ui/, tga.cpp, timer.cpp unbuilt.
- Smaller: uninitialised-field wording (GLM_FORCE_CTOR_INIT), `toa` 1.99999988, PCRE2 `$` vs Rust `$`, `--output-custom-log` needs non-empty path, shell no-update iteration, worker also does the `--start-date` scan.

Confirmed unchanged: min delta 16 ms (gource_shell.cpp:29), mapped `rand()` sites (4 plus `--start-position random`), regex wrapper semantics (regex.cpp matchOffset: offset>=size returns -1, trailing non-participating groups dropped, middle become empty), C++ `cvs-exp` unmatched by logmill (`"cvs"` only; fixed in Burl, sec. 10), conffile grammar regexes, parseArgs hyphen stripping, window height 640 on macOS.

Open: glibc/macOS `rand()` values unverified against a real libc; timing.md sec. 2 last paragraph ("only options present") and deps sec. 1 ("--version") are wrong in their own docs and were not edited.
