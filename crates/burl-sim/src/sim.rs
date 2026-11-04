//! `Gource` (`src/gource.cpp`) and the scene it owns. Arena ids replace the C++ pointers; every
//! `std::list` is a `Vec` in the same order (no swap-remove, ever), every `std::map` a `BTreeMap`
//! (byte order, like `std::string`). Order-sensitive spots are commented where they occur.
//!
//! The renderer-only calls in
//! `draw` have no simulation effect.

use burl_formats::{CommitLog, RCommit};
use burl_util::bounds::Bounds2D;
use burl_util::math::{normalise_vec2, rotate_vec2, PI};
use burl_util::prng::SimRng;
use burl_util::quadtree::QuadTree;
use burl_util::strhash::{colour_hash, vec2_hash};
use glam::{vec2, vec3, Vec2, Vec3};
use std::collections::{BTreeMap, VecDeque};

use crate::settings::{SimSettings, UserGraphic};

pub type FileId = usize;
pub type DirId = usize;
pub type UserId = usize;

#[derive(Debug)]
pub struct SimError(pub String);

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// gource.cpp / dirnode.cpp / file.cpp / user.cpp globals (all `float`).
const FILE_DIAMETER: f32 = 8.0;
/// `gGourceForceGravity`'s initial value; `[` / `]` scale it at run time.
const FORCE_GRAVITY: f32 = 10.0;
const DIR_PADDING: f32 = 1.5;
const BEAM_DIST: f32 = 100.0;
const ACTION_DIST: f32 = 50.0;
const PERSONAL_SPACE_DIST: f32 = 100.0;
const MAX_QUADTREE_DEPTH: i32 = 6;
/// `user.png` is 384x512 (`graphic->h / (float) graphic->w`); `file.png` is square, ratio 1.
const USER_GRAPHIC_RATIO: f32 = 512.0 / 384.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    Create,
    Remove,
    Modify,
}

#[derive(Clone, Debug)]
pub(crate) struct Action {
    pub kind: Kind,
    pub target: FileId,
    pub timestamp: i64,
    pub t: f32,
    pub progress: f32,
    pub rate: f32,
    pub colour: Vec3,
    pub modify_colour: Vec3,
}

pub(crate) struct File {
    pub fullpath: String,
    /// Directory part of `fullpath`, trailing `/` included.
    pub path: String,
    pub pos: Vec2,
    pub dest: Vec2,
    pub distance: f32,
    pub accel: Vec2,
    pub elapsed: f32,
    pub hidden: bool,
    pub removing: bool,
    pub expired: bool,
    pub forced_removal: bool,
    pub fade_start: f32,
    pub last_action: f32,
    pub removed_timestamp: i64,
    pub file_colour: Vec3,
    pub touch_colour: Vec3,
    pub size: f32,
    pub speed: f32,
    pub nametime: f32,
    pub name_interval: f32,
    pub tagid: i32,
    pub dir: Option<DirId>,
}

pub(crate) struct User {
    pub name: String,
    pub pos: Vec2,
    pub accel: Vec2,
    pub speed: f32,
    pub size: f32,
    pub elapsed: f32,
    pub last_action: f32,
    pub action_interval: f32,
    pub name_interval: f32,
    pub nametime: f32,
    pub usercol: Vec3,
    pub graphic_ratio: f32,
    pub graphic_texture: Option<u32>,
    pub highlighted: bool,
    pub actions: Vec<Action>,
    pub active_actions: Vec<Action>,
    /// `actionCount` / `activeCount`: counters kept apart from the lists, as in C++ (`fileRemoved`
    /// does not decrement `activeCount`, so the two can drift).
    pub action_count: i32,
    pub active_count: i32,
    pub tagid: i32,
    pub quad: Bounds2D,
}

pub(crate) struct Dir {
    pub abspath: String,
    pub parent: Option<DirId>,
    pub children: Vec<DirId>,
    pub files: Vec<FileId>,
    pub pos: Vec2,
    pub spos: Vec2,
    pub accel: Vec2,
    pub prev_accel: Vec2,
    pub dir_area: f32,
    pub file_area: f32,
    pub dir_radius: f32,
    pub parent_radius: f32,
    pub visible: bool,
    pub position_initialized: bool,
    pub visible_count: i32,
    pub depth: i32,
    pub since_node_visible: f32,
    pub since_last_file_change: f32,
    pub since_last_node_change: f32,
    pub col: [f32; 4],
    pub quad: Bounds2D,
}

/// `RCaption`: only the state the dump carries (layout and drawing are renderer matters).
pub(crate) struct Caption {
    pub text: String,
    pub timestamp: i64,
    pub elapsed: f32,
    pub alpha: f32,
}

pub(crate) struct Camera {
    pub pos: Vec3,
    pub dest: Vec3,
    pub target: Vec3,
    pub lockon: bool,
    pub lockon_time: f32,
    /// `_pos`/`_target`, restored by `ZoomCamera::reset`.
    pos0: Vec3,
    target0: Vec3,
    min_distance: f32,
    max_distance: f32,
    padding: f32,
    fov: f32,
    speed: f32,
}

impl Camera {
    /// `ZoomCamera(pos, target, min_distance, max_distance)` then `setPadding`.
    fn new(pos: Vec3, target: Vec3, min_distance: f32, max_distance: f32, padding: f32) -> Camera {
        Camera {
            pos,
            dest: pos,
            target,
            lockon: false,
            lockon_time: 0.0,
            pos0: pos,
            target0: target,
            min_distance,
            max_distance,
            padding,
            fov: 90.0,
            speed: 1.0,
        }
    }

    /// `ZoomCamera::adjust(bounds, adjust_distance)`.
    fn adjust(&mut self, bounds: &Bounds2D, s: &SimSettings, adjust_distance: bool) {
        let centre = bounds.centre();
        self.dest.x = centre.x;
        self.dest.y = centre.y;
        if !adjust_distance {
            return;
        }
        let mut width = bounds.width() * self.padding;
        let mut height = bounds.height() * self.padding;
        let aspect_ratio = s.display_width as f32 / s.display_height as f32;
        if aspect_ratio < 1.0 {
            height /= aspect_ratio;
        } else {
            width /= aspect_ratio;
        }
        // `tan(fov * 0.5f * DEGREES_TO_RADIANS) * 2.0`: the product is a double, libm `tan` is double.
        let toa =
            (((self.fov * 0.5) as f64 * burl_util::math::DEGREES_TO_RADIANS).tan() * 2.0) as f32;
        let mut distance = if s.crop_vertical {
            width / toa
        } else if s.crop_horizontal {
            height / toa
        } else if width >= height {
            width / toa
        } else {
            height / toa
        };
        if distance < self.min_distance {
            distance = self.min_distance;
        }
        if distance > self.max_distance {
            distance = self.max_distance;
        }
        self.dest.z = -distance;
    }

    /// `ZoomCamera::setPos(pos, keep_angle = true)`.
    fn set_pos_keep_angle(&mut self, pos: Vec3) {
        let dir = self.target - self.pos;
        self.pos = pos;
        self.target = pos + dir;
    }

    /// `ZoomCamera::lockOn`.
    fn lock_on(&mut self, lockon: bool) {
        if lockon {
            self.lockon_time = 1.0;
        }
        self.lockon = lockon;
    }

    /// `ZoomCamera::stop`.
    fn stop(&mut self) {
        self.dest = self.pos;
    }

    /// `ZoomCamera::logic`.
    fn logic(&mut self, dt: f32) {
        let dp = self.dest - self.pos;
        let mut dpt = dp * dt * self.speed;
        if self.lockon {
            dpt = dpt * self.lockon_time + dp * (1.0 - self.lockon_time);
            if self.lockon_time > 0.0 {
                self.lockon_time = f32::max(0.0, self.lockon_time - dt * 0.5);
            }
        }
        if dpt.length_squared() > dp.length_squared() {
            dpt = dp;
        }
        self.pos += dpt;
        self.target = vec3(self.pos.x, self.pos.y, 0.0);
    }
}

/// `rotate_vec2` about the origin, or about `centre` (kept apart so the origin case does not
/// pick up `x - 0 + 0`, which turns a `-0.0` into `+0.0`).
fn rotate_about(v: Vec2, s: f32, c: f32, centre: Option<Vec2>) -> Vec2 {
    match centre {
        None => rotate_vec2(v, s, c),
        Some(o) => rotate_vec2(v - o, s, c) + o,
    }
}

pub struct Sim {
    pub(crate) s: SimSettings,
    rng: SimRng,
    pub(crate) log: Box<dyn CommitLog>,

    pub(crate) framecount: i32,
    pub(crate) runtime: f32,
    max_tick_rate: f32,
    /// `Gource::frameskip`: ticks per exported frame minus one.
    frameskip: i32,
    pub(crate) currtime: i64,
    pub(crate) lasttime: i64,
    pub(crate) subseconds: f32,
    pub(crate) idle_time: f32,
    pub(crate) stop_position_reached: bool,
    first_read: bool,
    pub(crate) commitqueue: VecDeque<RCommit>,
    commitqueue_max_size: usize,
    pub(crate) last_percent: f32,
    pub(crate) rotation_remaining_angle: f32,
    pub(crate) rotate_angle: f32,
    /// `captions` (queued, in file order) and `active_captions`.
    pub(crate) captions: VecDeque<Caption>,
    pub(crate) active_captions: Vec<Caption>,
    /// How many captions have become active so far (the renderer's row bookkeeping key).
    pub(crate) captions_activated: usize,
    /// `file_key` calls waiting for the renderer (`SceneView::key_events`).
    pub(crate) key_events: Vec<crate::view::KeyEvent>,
    pub(crate) tag_seq: i32,
    pub(crate) user_bounds: Bounds2D,
    active_user_bounds: Bounds2D,
    pub(crate) dir_bounds: Bounds2D,
    pub(crate) camera: Camera,
    /// `track_users` (`--camera-mode track`) and `selectedUser`.
    track_users: bool,
    pub(crate) selected_user: Option<UserId>,
    /// `selectedFile`: only one of the two is ever set.
    pub(crate) selected_file: Option<FileId>,
    /// `hoverUser` / `hoverFile` (`mousetrace`).
    pub(crate) hover_user: Option<UserId>,
    pub(crate) hover_file: Option<FileId>,
    /// `mousepos`, `mousedragged`, `grab_mouse` and the held-button mask (`cursor`).
    pub(crate) mousepos: Vec2,
    pub(crate) mousedragged: bool,
    pub(crate) grab_mouse: bool,
    pub(crate) pointer_buttons: i32,
    /// Dump the `g.pointer` object (a pointer script is running).
    pub(crate) pointer_dump: bool,
    /// Set by a zoom command (`manual_zoom`): the camera stops fitting the bounds' distance.
    pub(crate) manual_zoom: bool,
    /// Input state (T2.2): `paused`, `manual_camera`, `manual_rotate`, `cursor_move`,
    /// `gGourceForceGravity`.
    pub(crate) paused: bool,
    pub(crate) manual_camera: bool,
    pub(crate) manual_rotate: bool,
    pub(crate) cursor_move: Vec2,
    pub(crate) force_gravity: f32,
    /// `cursor.rightButtonPressed()`.
    pub(crate) right_button: bool,
    pub(crate) app_finished: bool,
    /// `file_idle_time` is overwritten once the log ends (`file_idle_time_at_end`).
    file_idle_time: f32,

    pub(crate) first_rand_frame: i64,

    files: Vec<Option<File>>,
    pub(crate) file_map: BTreeMap<String, FileId>,
    users: Vec<Option<User>>,
    pub(crate) user_map: BTreeMap<String, UserId>,
    dirs: Vec<Option<Dir>>,
    /// `gGourceDirMap`.
    pub(crate) dir_map: BTreeMap<String, DirId>,
    pub(crate) root: DirId,
    /// `gGourceRemovedFiles`.
    pub(crate) removed: Vec<FileId>,

    pub(crate) loaded_frame0: i32,
    pub(crate) runtime_at_load: f32,
}

fn f32_max(a: f32, b: f32) -> f32 {
    f32::max(a, b)
}

impl Sim {
    /// `Gource::Gource` with a frame exporter (fixed tick rate). The log is already loaded
    /// (deviation D2), so the first update is the first loaded frame.
    pub fn new(
        settings: SimSettings,
        log: Box<dyn CommitLog>,
        rng: SimRng,
    ) -> Result<Sim, SimError> {
        let track_users = settings.track_users;

        // setFrameExporter: tick rate for the video frame rate
        let mut gource_framerate = settings.output_framerate;
        let mut frameskip = 0;
        while gource_framerate < 60 {
            gource_framerate += settings.output_framerate;
            frameskip += 1;
        }
        let max_tick_rate = 1.0f32 / gource_framerate as f32;

        let camera = Camera::new(
            vec3(0.0, 0.0, -300.0),
            Vec3::ZERO,
            settings.camera_zoom_default,
            settings.camera_zoom_max,
            settings.padding,
        );
        let file_idle_time = settings.file_idle_time;
        let mut sim = Sim {
            s: settings,
            rng,
            log,
            framecount: 0,
            runtime: 0.0,
            max_tick_rate,
            frameskip,
            currtime: 0,
            lasttime: 0,
            subseconds: 0.0,
            idle_time: 0.0,
            stop_position_reached: false,
            first_read: true,
            commitqueue: VecDeque::new(),
            commitqueue_max_size: 100,
            last_percent: 0.0,
            rotation_remaining_angle: 0.0,
            rotate_angle: 0.0,
            captions: VecDeque::new(),
            active_captions: Vec::new(),
            captions_activated: 0,
            key_events: Vec::new(),
            tag_seq: 1,
            user_bounds: Bounds2D::default(),
            active_user_bounds: Bounds2D::default(),
            dir_bounds: Bounds2D::default(),
            camera,
            // setCameraMode(gGourceSettings.camera_mode): nothing is selected yet, so no lock-on
            track_users,
            selected_user: None,
            selected_file: None,
            hover_user: None,
            hover_file: None,
            mousepos: Vec2::ZERO,
            mousedragged: false,
            grab_mouse: false,
            pointer_buttons: 0,
            pointer_dump: false,
            manual_zoom: false,
            paused: false,
            manual_camera: false,
            manual_rotate: false,
            cursor_move: Vec2::ZERO,
            force_gravity: FORCE_GRAVITY,
            right_button: false,
            app_finished: false,
            file_idle_time,
            first_rand_frame: -1,
            files: Vec::new(),
            file_map: BTreeMap::new(),
            users: Vec::new(),
            user_map: BTreeMap::new(),
            dirs: Vec::new(),
            dir_map: BTreeMap::new(),
            root: 0,
            removed: Vec::new(),
            loaded_frame0: -1,
            runtime_at_load: 0.0,
        };
        // reset(): root = new RDirNode(0, "/")
        sim.root = sim.dir_new("/");
        // GourceSettings::importGourceSettings: `start_position = (rand() % 1000) / 1000.0f`. The
        // C++ reseeds with `srand(time(0))` first; the default generator is already clock-seeded
        // and the fork's `gource_srand_time` is a no-op for every other mode, so only the draw remains.
        // The validating import in `main.cpp` draws first; `run_state_dump` replays that one.
        if sim.s.start_position_random {
            sim.s.start_position = (sim.rand() % 1000) as f32 / 1000.0;
        }
        // Gource::update, once the log is ready: `if(start_position>0.0) seekTo(start_position)`
        if sim.s.start_position > 0.0 {
            sim.seek_to(sim.s.start_position);
        }
        Ok(sim)
    }

    /// `last_percent`: where in the log the commits read so far end (the slider's position).
    pub fn percent(&self) -> f32 {
        self.last_percent
    }

    /// The `dt` `Gource::logic` sees for a real frame time `dt` (`step`: capped, then scaled).
    pub fn logic_dt(&self, dt: f32) -> f32 {
        dt.min(self.max_tick_rate) * self.s.time_scale
    }

    /// `Gource::dateAtPosition`: the timestamp of the commit under `percent` of the log.
    pub fn commit_time_at(&mut self, percent: f32) -> Option<i64> {
        if percent >= 1.0 {
            return None;
        }
        self.log.get_commit_at(percent).map(|c| c.timestamp)
    }

    /// `Gource::seekTo`: ignored for a log that cannot seek.
    pub(crate) fn seek_to(&mut self, percent: f32) {
        if !self.log.is_seekable() {
            return;
        }
        // end pause
        self.paused = false;
        self.reset();
        self.log.seek_to(percent);
    }

    /// `Gource::reset`. Not reset, as in the C++: the runtime and frame counters, `first_read`,
    /// `stop_position_reached`, `file_idle_time` and the PRNG.
    fn reset(&mut self) {
        self.camera.pos = self.camera.pos0;
        self.camera.target = self.camera.target0;
        self.user_bounds = Bounds2D::default();
        self.active_user_bounds = Bounds2D::default();
        self.dir_bounds = Bounds2D::default();
        self.commitqueue.clear();
        self.removed.clear();
        self.manual_rotate = false;
        self.manual_zoom = false;
        self.rotation_remaining_angle = 0.0;
        self.cursor_move = Vec2::ZERO;
        self.selected_user = None;
        self.selected_file = None;
        self.hover_user = None;
        self.hover_file = None;
        self.grab_mouse = false;
        self.mousedragged = false;
        self.manual_camera = false;
        self.commitqueue_max_size = 100;
        self.rotate_angle = 0.0;
        self.files.clear();
        self.file_map.clear();
        self.users.clear();
        self.user_map.clear();
        self.dirs.clear();
        self.dir_map.clear();
        self.root = self.dir_new("/");
        self.captions.clear();
        self.active_captions.clear();
        self.last_percent = 0.0;
        self.key_events.push(crate::view::KeyEvent::Clear);
        self.idle_time = 0.0;
        self.currtime = 0;
        self.lasttime = 0;
        self.subseconds = 0.0;
        self.tag_seq = 1;
    }

    /// The generator and the fork's global `first_rand_frame`, which outlive a section: the next
    /// section's Sim continues the same stream (`rand_calls`, script position, libc state).
    pub fn into_carry(self) -> (SimRng, i64) {
        (self.rng, self.first_rand_frame)
    }

    /// [`Self::into_carry`] for a Sim that stays in place: the generator is replaced by a fresh
    /// one, so this Sim must be dropped without being stepped again.
    pub fn take_carry(&mut self) -> (SimRng, i64) {
        let spent = SimRng::new(burl_util::prng::PrngMode::Glibc, None);
        (
            std::mem::replace(&mut self.rng, spent),
            self.first_rand_frame,
        )
    }

    /// Resume the carried `first_rand_frame` (set once, by the first `rand()` of the whole run).
    pub fn carry_first_rand_frame(&mut self, f: i64) {
        // a draw made by `Sim::new` (`--start-position random`) stays if no earlier section drew
        if f >= 0 {
            self.first_rand_frame = f;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.app_finished
    }

    pub fn rand_calls(&self) -> u64 {
        self.rng.calls()
    }

    pub fn rand_last(&self) -> i32 {
        self.rng.last()
    }

    /// `gource_rand()`: the fork counts the frame of the first draw.
    fn rand(&mut self) -> i32 {
        let v = self.rng.next_i31();
        if self.first_rand_frame < 0 {
            self.first_rand_frame = self.framecount as i64;
        }
        v
    }

    // ---- arenas ------------------------------------------------------------------------

    pub(crate) fn file(&self, id: FileId) -> &File {
        self.files[id].as_ref().expect("live file id")
    }
    fn file_mut(&mut self, id: FileId) -> &mut File {
        self.files[id].as_mut().expect("live file id")
    }
    pub(crate) fn user(&self, id: UserId) -> &User {
        self.users[id].as_ref().expect("live user id")
    }
    fn user_mut(&mut self, id: UserId) -> &mut User {
        self.users[id].as_mut().expect("live user id")
    }
    pub(crate) fn dir(&self, id: DirId) -> &Dir {
        self.dirs[id].as_ref().expect("live dir id")
    }
    fn dir_mut(&mut self, id: DirId) -> &mut Dir {
        self.dirs[id].as_mut().expect("live dir id")
    }

    // ---- Gource::update / logic --------------------------------------------------------

    /// `Gource::update` minus the draw: one fixed-`dt` tick. The caller dumps between this
    /// and the next call, as the fork's hook sits after `logic`.
    pub fn update(&mut self) -> Result<(), SimError> {
        self.step(self.max_tick_rate)
    }

    /// The interactive tick: `scaled_dt = min(dt, max_tick_rate)` for the wall-clock `dt` of the
    /// frame (`Gource::update`); [`Sim::update`] is the exporter's fixed tick.
    pub fn update_realtime(&mut self, dt: f32) -> Result<(), SimError> {
        self.step(dt.min(self.max_tick_rate))
    }

    fn step(&mut self, tick: f32) -> Result<(), SimError> {
        // exporter: scaled_dt = max_tick_rate; then time scale
        let mut scaled_dt = tick;
        scaled_dt *= self.s.time_scale;

        // runtime is managed internally
        if !self.paused {
            self.runtime += scaled_dt;
        }

        if self.s.stop_at_time > 0.0 && self.runtime >= self.s.stop_at_time {
            self.stop_position_reached = true;
        }

        let runtime = self.runtime;
        self.logic(runtime, scaled_dt)?;

        if self.loaded_frame0 < 0 {
            self.loaded_frame0 = self.framecount;
            self.runtime_at_load = self.runtime;
        }
        Ok(())
    }

    /// `Gource::update`'s `framecount % (frameskip + 1) == 0`: whether the exporter dumps this tick.
    pub fn exports_frame(&self) -> bool {
        self.framecount % (self.frameskip + 1) == 0
    }

    /// Called by the driver after the dump hook, in place of `Gource::update`'s tail.
    pub fn end_frame(&mut self) {
        self.framecount += 1;
    }

    fn logic(&mut self, t: f32, dt: f32) -> Result<(), SimError> {
        self.key_events.push(crate::view::KeyEvent::Tick(dt));
        self.apply_rotation();

        // still want to update the camera while paused
        if self.paused {
            self.update_bounds();
            self.interact_users();
            self.interact_dirs();
            self.update_camera(dt);
            return Ok(());
        }

        // get more entries
        if self.commitqueue.is_empty() {
            self.read_log()?;
        }

        // loop in attempt to find commits
        if self.s.loop_
            && self.commitqueue.is_empty()
            && self.log.is_seekable()
            && self.idle_time >= self.s.loop_delay_seconds
        {
            self.first_read = true;
            self.seek_to(0.0);
            self.read_log()?;
        }

        if self.currtime == 0 && !self.commitqueue.is_empty() {
            self.currtime = self.commitqueue[0].timestamp;
            self.lasttime = self.currtime;
            self.subseconds = 0.0;
            self.load_captions();
        }

        // set current time: `float time_inc = (dt * 86400.0 * days_per_second)` (double math)
        let time_inc = (dt as f64 * 86400.0 * self.s.days_per_second as f64) as f32;
        let seconds = time_inc as i32;

        self.subseconds += time_inc - (seconds as f32);

        if self.subseconds >= 1.0 {
            self.currtime += (self.subseconds as i32) as i64;
            self.subseconds -= (self.subseconds as i32) as f32;
        }

        self.currtime += seconds as i64;

        // delete files
        let removed = std::mem::take(&mut self.removed);
        for f in removed {
            self.delete_file(f);
        }

        // add commits up until the current time
        while let Some(commit) = self.commitqueue.front().cloned() {
            // auto skip ahead, unless stop_position_reached
            if self.s.auto_skip_seconds >= 0.0
                && self.idle_time >= self.s.auto_skip_seconds
                && !self.stop_position_reached
            {
                self.currtime = commit.timestamp;
                self.lasttime = commit.timestamp;
                self.idle_time = 0.0;
            }

            if commit.timestamp > self.currtime {
                break;
            }

            self.process_commit(&commit, t);

            if self.s.no_time_travel {
                if commit.timestamp > self.lasttime {
                    self.lasttime = commit.timestamp;
                }
            } else {
                // allow for non linear time lines
                if self.lasttime > commit.timestamp {
                    self.currtime = commit.timestamp;
                }
                self.lasttime = commit.timestamp;
            }

            self.subseconds = 0.0;
            self.commitqueue.pop_front();
        }

        self.update_captions(dt);

        self.update_bounds();
        self.interact_users();
        self.update_users(t, dt);
        let tree = self.interact_dirs();
        self.update_dirs(dt, &tree);
        self.update_camera(dt);
        Ok(())
    }

    fn read_log(&mut self) -> Result<(), SimError> {
        if self.stop_position_reached {
            return Ok(());
        }

        while (self.log.has_buffered_commit() || !self.log.is_finished())
            && (self.commitqueue.is_empty()
                || (self
                    .commitqueue
                    .back()
                    .is_some_and(|c| c.timestamp <= self.currtime)
                    && self.commitqueue.len() < self.commitqueue_max_size))
        {
            let mut commit = RCommit::default();
            if !self.log.next_commit(&mut commit, true) {
                if !self.log.is_seekable() {
                    break;
                }
                continue;
            }
            if self.s.stop_timestamp != 0 && commit.timestamp > self.s.stop_timestamp {
                self.stop_position_reached = true;
                break;
            }
            self.commitqueue.push_back(commit);
        }

        if self.first_read && self.commitqueue.is_empty() {
            return Err(SimError("no commits found".into()));
        }
        self.first_read = false;

        if !self.log.is_finished() && self.log.is_seekable() {
            self.last_percent = self.log.get_percent();
        }

        let is_finished = self.log.is_finished();

        if (self.s.stop_at_end && is_finished)
            || (self.s.stop_position > 0.0
                && self.log.is_seekable()
                && (is_finished || self.last_percent >= self.s.stop_position))
        {
            self.stop_position_reached = true;
        }

        if (is_finished || self.stop_position_reached) && self.s.file_idle_time_at_end > 0.0 {
            self.file_idle_time = self.s.file_idle_time_at_end;
        }
        Ok(())
    }

    // ---- commits -----------------------------------------------------------------------

    fn process_commit(&mut self, commit: &RCommit, t: f32) {
        for cf in &commit.files {
            // is this a directory (ends in slash)
            if cf.filename.ends_with('/') {
                // ignore unless it is a delete: a directory is not a physical entity
                if cf.action != "D" {
                    continue;
                }
                let mut dirs = Vec::new();
                self.dir_find_dirs(self.root, &cf.filename, &mut dirs);
                for d in dirs {
                    let mut dir_files = Vec::new();
                    self.dir_files_recursive(d, &mut dir_files);
                    for f in dir_files {
                        self.add_file_action(commit, &cf.action, cf.colour, f, t);
                    }
                }
                continue;
            }

            let file = match self.file_map.get(&cf.filename) {
                Some(&f) => f,
                None => match self.add_file(&cf.filename, cf.colour) {
                    Some(f) => f,
                    None => continue,
                },
            };
            self.add_file_action(commit, &cf.action, cf.colour, file, t);
        }
    }

    fn add_file(&mut self, filename: &str, colour: [f32; 3]) -> Option<FileId> {
        // if we already have max files in circulation we cant add any more
        if self.s.max_files > 0 && self.file_map.len() >= self.s.max_files as usize {
            return None;
        }

        let mut file_as_dir = filename.to_string();
        if !file_as_dir.ends_with('/') {
            file_as_dir.push('/');
        }
        if self.dir_is_dir(self.root, &file_as_dir) {
            return None;
        }

        let tagid = self.tag_seq;
        self.tag_seq += 1;

        // RFile::RFile
        let size = (FILE_DIAMETER as f64 * 1.05) as f32;
        let nametime = self.s.filename_time;
        let path = match filename.rfind('/') {
            Some(p) => filename[..=p].to_string(),
            None => String::new(),
        };
        let file = File {
            fullpath: filename.to_string(),
            path,
            pos: Vec2::ZERO,
            dest: Vec2::ZERO,
            distance: 0.0,
            accel: Vec2::ZERO,
            elapsed: 0.0,
            hidden: true,
            removing: false,
            expired: false,
            forced_removal: false,
            fade_start: -1.0,
            last_action: 0.0,
            removed_timestamp: 0,
            file_colour: Vec3::from_array(colour),
            touch_colour: Vec3::ZERO,
            size,
            speed: 5.0,
            nametime,
            name_interval: nametime,
            tagid,
            dir: None,
        };
        let id = self.files.len();
        self.files.push(Some(file));
        self.file_map.insert(filename.to_string(), id);
        self.key_events.push(crate::view::KeyEvent::Inc {
            path: filename.to_string(),
            colour: Vec3::from_array(colour),
        });

        let root = self.root;
        self.dir_add_file(root, id);

        while let Some(p) = self.dir(self.root).parent {
            self.root = p;
        }
        Some(id)
    }

    fn delete_file(&mut self, f: FileId) {
        let root = self.root;
        self.dir_remove_file(root, f);

        // remove from any users with actions against this file
        let uids: Vec<UserId> = self.user_map.values().copied().collect();
        for u in uids {
            self.user_file_removed(u, f);
        }

        if self.hover_file == Some(f) {
            self.hover_file = None;
        }
        if self.selected_file == Some(f) {
            self.select_file(None);
        }

        let fullpath = self.file(f).fullpath.clone();
        self.file_map.remove(&fullpath);
        self.key_events
            .push(crate::view::KeyEvent::Dec { path: fullpath });
        self.files[f] = None;
    }

    fn add_file_action(
        &mut self,
        commit: &RCommit,
        action: &str,
        colour: [f32; 3],
        file: FileId,
        t: f32,
    ) {
        let username = commit.username_str().to_string();
        let uid = match self.user_map.get(&username) {
            Some(&u) => u,
            None => {
                let u = self.add_user(&username);
                if self.s.highlight_all_users {
                    self.user_mut(u).highlighted = true;
                } else {
                    // highlighted if the name matches a highlighted user
                    for hi in &self.s.highlight_users {
                        if !hi.is_empty() && username == *hi {
                            self.users[u].as_mut().expect("just added").highlighted = true;
                            break;
                        }
                    }
                }
                u
            }
        };

        let (kind, base) = match action {
            "D" => (Kind::Remove, vec3(1.0, 0.0, 0.0)),
            "A" => (Kind::Create, vec3(0.0, 1.0, 0.0)),
            _ => (Kind::Modify, vec3(1.0, 0.7, 0.3)),
        };
        let a = Action {
            kind,
            target: file,
            timestamp: commit.timestamp,
            t,
            progress: 0.0,
            rate: 0.5,
            colour: base,
            modify_colour: Vec3::from_array(colour),
        };
        // RUser::addAction
        let u = self.user_mut(uid);
        let idle = u.active_actions.is_empty() && u.actions.is_empty();
        if idle && u.name_interval <= 0.0 {
            u.name_interval = u.nametime;
        }
        u.actions.push(a);
        u.action_count += 1;
    }

    fn add_user(&mut self, username: &str) -> UserId {
        let pos = if self.dir_bounds.area() > 0.0 {
            self.dir_bounds.centre()
        } else {
            Vec2::ZERO
        };
        let tagid = self.tag_seq;
        self.tag_seq += 1;

        // RUser::RUser + assignUserImage: colourHash (white for an avatar unless --colour-images),
        // then lighten and dim
        let g = self
            .s
            .user_graphics
            .as_ref()
            .and_then(|h| (h.0)(username))
            .unwrap_or(UserGraphic {
                ratio: USER_GRAPHIC_RATIO,
                texture: None,
                white: false,
            });
        let usercol = if g.white {
            Vec3::ONE
        } else {
            Vec3::from_array(colour_hash(username.as_bytes(), self.s.hash_seed))
        };
        let usercol = usercol * 0.6 + Vec3::splat(1.0) * 0.4;
        let usercol = usercol * 0.9;

        let user = User {
            name: username.to_string(),
            pos,
            accel: Vec2::ZERO,
            speed: self.s.max_user_speed,
            size: (20.0f64 * self.s.user_scale as f64) as f32,
            elapsed: 0.0,
            last_action: 0.0,
            action_interval: 0.2f64 as f32,
            name_interval: 5.0,
            nametime: 5.0,
            usercol,
            graphic_ratio: g.ratio,
            graphic_texture: g.texture,
            highlighted: false,
            actions: Vec::new(),
            active_actions: Vec::new(),
            action_count: 0,
            active_count: 0,
            tagid,
            quad: Bounds2D::default(),
        };
        let id = self.users.len();
        self.users.push(Some(user));
        self.user_map.insert(username.to_string(), id);
        id
    }

    // ---- bounds, users ------------------------------------------------------------------

    fn update_bounds(&mut self) {
        self.user_bounds.reset();
        self.active_user_bounds.reset();

        let uids: Vec<UserId> = self.user_map.values().copied().collect();
        for uid in uids {
            // Pawn::updateQuadItemBounds
            let u = self.user_mut(uid);
            let halfsize_x = u.size * 0.5;
            let halfsize = vec2(halfsize_x, halfsize_x * u.graphic_ratio);
            u.quad = Bounds2D::new(u.pos - halfsize, u.pos + halfsize);
            let quad = u.quad;
            let idle = u.active_actions.is_empty() && u.actions.is_empty();
            self.user_bounds.update_bounds(&quad);
            if !idle {
                self.active_user_bounds.update_bounds(&quad);
            }
        }

        self.dir_bounds.reset();
        // gGourceDirMap order: sorted by path, so a parent is tested before its children
        let dids: Vec<DirId> = self.dir_map.values().copied().collect();
        for d in dids {
            if self.dir_is_visible(d) {
                let radius = self.dir(d).dir_radius;
                let radoffset = vec2(radius, radius);
                let pos = self.dir(d).pos;
                let q = Bounds2D::new(pos - radoffset, pos + radoffset);
                self.dir_mut(d).quad = q;
                self.dir_bounds.update_bounds(&q);
            }
        }
    }

    fn interact_users(&mut self) {
        let mut b = self.user_bounds;
        b.min -= vec2(1.0, 1.0);
        b.max += vec2(1.0, 1.0);

        // dont use deep quad tree initially when all the nodes are in one place
        let max_depth = if self.dir_bounds.area() > 10000.0 {
            MAX_QUADTREE_DEPTH
        } else {
            1
        };
        let mut tree: QuadTree<UserId> = QuadTree::new(b, max_depth, 1);

        let uids: Vec<UserId> = self.user_map.values().copied().collect();
        for &u in &uids {
            tree.add_item(u, self.user(u).quad);
        }

        // move users - interact with other users and files
        for &a in &uids {
            // UserForceFunctor: skip self, dedupe items that sit in several leaves
            let mut seen: Vec<UserId> = Vec::new();
            let mut visit: Vec<UserId> = Vec::new();
            let quad = self.user(a).quad;
            tree.visit_items_in_bounds(&quad, |b_id, node_count| {
                if b_id == a {
                    return;
                }
                if node_count != 1 {
                    if seen.contains(&b_id) {
                        return;
                    }
                    seen.push(b_id);
                }
                visit.push(b_id);
            });
            for b_id in visit {
                self.user_apply_force_user(a, b_id);
            }
            self.user_apply_force_to_actions(a);
        }
    }

    fn user_apply_force_user(&mut self, a: UserId, u: UserId) {
        if u == a {
            return;
        }
        let u_pos = self.user(u).pos;
        let me = self.user(a);
        let dir = u_pos - me.pos;
        let dist = dir.length();

        // different repelling force depending on how busy the user is (double math, then float)
        let action_count = me.action_count + me.active_count;
        let desired_dist = (if action_count == 0 {
            PERSONAL_SPACE_DIST as f64
        } else if !me.actions.is_empty() && me.active_actions.is_empty() {
            PERSONAL_SPACE_DIST as f64 * 0.1
        } else {
            PERSONAL_SPACE_DIST as f64 * 0.5
        }) as f32;

        // resolve overlap
        if (dist as f64) < 0.001 {
            let rx = self.rand() % 100;
            let ry = self.rand() % 100;
            let v = normalise_vec2(vec2((rx - 50) as f32, (ry - 50) as f32));
            self.user_mut(a).accel += 1.0f32 * v;
            return;
        }

        // repelling force
        if dist < desired_dist {
            self.user_mut(a).accel -= (desired_dist - dist) * normalise_vec2(dir);
        }
    }

    fn user_apply_force_action(&mut self, a: UserId, target: FileId) {
        let f_pos = self.file_abs_pos(target);
        let pos = self.user(a).pos;
        let dir = f_pos - pos;
        let dist = dir.length();

        let desired_dist = ACTION_DIST;

        // resolve overlap
        if (dist as f64) < 0.001 {
            let rx = self.rand() % 100;
            let ry = self.rand() % 100;
            let v = normalise_vec2(vec2((rx - 50) as f32, (ry - 50) as f32));
            self.user_mut(a).accel += v;
            return;
        }

        // repelling force
        if dist < desired_dist {
            self.user_mut(a).accel -= (desired_dist - dist) * normalise_vec2(dir);
            return;
        }

        if dist > BEAM_DIST {
            self.user_mut(a).accel += (dist - BEAM_DIST) * normalise_vec2(dir);
        }
    }

    fn user_apply_force_to_actions(&mut self, a: UserId) {
        let u = self.user(a);
        if u.active_actions.is_empty() && u.actions.is_empty() {
            return;
        }
        let elapsed = u.elapsed;
        self.user_mut(a).last_action = elapsed;

        let max_influence = 3;
        let mut action_influence = 0;

        // move towards actions being worked on
        let targets: Vec<FileId> = self
            .user(a)
            .active_actions
            .iter()
            .map(|x| x.target)
            .collect();
        for target in &targets {
            self.user_apply_force_action(a, *target);
            action_influence += 1;
            if action_influence >= max_influence {
                break;
            }
        }

        if !targets.is_empty() {
            return;
        }

        // if no actions being worked on, move towards one pending action
        if let Some(first) = self.user(a).actions.first().map(|x| x.target) {
            self.user_apply_force_action(a, first);
        }
    }

    /// `RUser::fileRemoved`.
    fn user_file_removed(&mut self, uid: UserId, f: FileId) {
        let u = self.user_mut(uid);
        let before = u.actions.len();
        u.actions.retain(|a| a.target != f);
        u.action_count -= (before - u.actions.len()) as i32;
        // the active list is erased without touching activeCount (C++ behaviour)
        u.active_actions.retain(|a| a.target != f);
    }

    fn update_users(&mut self, t: f32, dt: f32) {
        let mut inactive: Vec<UserId> = Vec::new();
        let mut idle_users = 0usize;

        let uids: Vec<UserId> = self.user_map.values().copied().collect();
        for &uid in &uids {
            self.user_logic(uid, t, dt);

            let u = self.user(uid);
            let idle = u.active_actions.is_empty() && u.actions.is_empty();
            // deselect a user fading out from inactivity
            if idle
                && ((u.elapsed - u.last_action) > self.s.user_idle_time)
                && self.selected_user == Some(uid)
            {
                self.select_user(None);
            }
            let u = self.user(uid);
            // isInactive
            if idle && ((u.elapsed - u.last_action) as f64) > 10.0 {
                inactive.push(uid);
            }
            if idle {
                idle_users += 1;
            } else if self.selected_user.is_none() && self.selected_file.is_none() {
                // nothing is selected, and this active user is one to follow
                let name = &self.user(uid).name;
                if self
                    .s
                    .follow_users
                    .iter()
                    .any(|f| !f.is_empty() && f == name)
                {
                    self.select_user(Some(uid));
                }
            }
        }

        if self.user_map.is_empty() && self.stop_position_reached {
            self.app_finished = true;
        }

        // nothing is moving so increment idle
        if idle_users == self.user_map.len() {
            self.idle_time += dt;
        } else {
            self.idle_time = 0.0;
        }

        // delete inactive users
        for uid in inactive {
            if self.hover_user == Some(uid) {
                self.hover_user = None;
            }
            if self.selected_user == Some(uid) {
                self.select_user(None);
            }
            let name = self.user(uid).name.clone();
            self.user_map.remove(&name);
            self.users[uid] = None;
        }
    }

    /// `Gource::selectUser`: deselects the file and the user, then selects `user` and, in track
    /// mode, locks the camera on.
    pub(crate) fn select_user(&mut self, user: Option<UserId>) {
        // already selected: nothing to do
        if user.is_some() && self.selected_user == user {
            return;
        }
        self.selected_file = None;
        self.selected_user = user;
        match user {
            None => self.camera.lock_on(false),
            Some(_) if self.track_users => self.camera.lock_on(true),
            Some(_) => {}
        }
    }

    /// `Gource::selectFile`: deselects the user and the file; no camera lock.
    pub(crate) fn select_file(&mut self, file: Option<FileId>) {
        if file.is_some() && self.selected_file == file {
            return;
        }
        self.selected_user = None;
        self.selected_file = file;
    }

    /// `Gource::selectNextUser` (Tab): the next user in tag-id order after the selected one that
    /// is not inactive and fully opaque, else the first such user.
    pub(crate) fn select_next_user(&mut self) {
        let curr = self.selected_user.map_or(-1, |u| self.user(u).tagid);
        // `tagusermap` is a std::map<int, RUser*>
        let mut by_tag: Vec<(i32, UserId)> = self
            .user_map
            .values()
            .map(|&u| (self.user(u).tagid, u))
            .collect();
        by_tag.sort_unstable();
        let ok = |s: &Sim, u: UserId| !s.user_is_inactive(u) && s.user_alpha(u) >= 1.0;
        let next = by_tag
            .iter()
            .find(|&&(tag, u)| tag > curr && ok(self, u))
            .or_else(|| by_tag.iter().find(|&&(_, u)| ok(self, u)))
            .map(|&(_, u)| u);
        self.select_user(next);
    }

    /// `RUser::isInactive`.
    pub(crate) fn user_is_inactive(&self, u: UserId) -> bool {
        let user = self.user(u);
        user.active_actions.is_empty()
            && user.actions.is_empty()
            && (user.elapsed - user.last_action) as f64 > 10.0
    }

    /// `RUser::logic`.
    fn user_logic(&mut self, uid: UserId, t: f32, dt: f32) {
        // Pawn::logic (users are never hidden)
        {
            let u = self.user_mut(uid);
            u.elapsed += dt;
            if u.name_interval > 0.0 {
                u.name_interval -= dt;
            }
            u.action_interval -= dt;
        }

        let find_nearby_action = {
            let u = self.user(uid);
            !u.actions.is_empty() && u.action_interval <= 0.0
        };

        // add next active action, if it is in range
        let mut i = 0;
        while i < self.user(uid).actions.len() {
            let (a_t, target) = {
                let a = &self.user(uid).actions[i];
                (a.t, a.target)
            };

            // add all files which are too old
            if self.s.max_file_lag >= 0.0 && a_t < t - self.s.max_file_lag {
                let u = self.user_mut(uid);
                let mut a = u.actions.remove(i);
                u.action_count -= 1;
                a.rate = 2.0;
                u.active_actions.push(a);
                u.active_count += 1;
                continue;
            }

            if !find_nearby_action {
                break;
            }

            let action_dist = (self.file_abs_pos(target) - self.user(uid).pos).length();

            // queue first action in range
            if action_dist < BEAM_DIST {
                let u = self.user_mut(uid);
                let a = u.actions.remove(i);
                u.active_actions.push(a);
                u.action_count -= 1;
                u.active_count += 1;
                break;
            }

            i += 1;
        }

        // reset action interval
        if self.user(uid).action_interval <= 0.0 {
            let u = self.user_mut(uid);
            let total_actions = u.action_count + u.active_count;
            u.action_interval = if total_actions != 0 {
                (1.0f64 / (total_actions as f32) as f64) as f32
            } else {
                1.0
            };
        }

        // update actions
        let mut i = 0;
        while i < self.user(uid).active_actions.len() {
            self.action_logic(uid, i, dt);
            if self.user(uid).active_actions[i].progress >= 1.0 {
                let u = self.user_mut(uid);
                u.active_actions.remove(i);
                u.active_count -= 1;
                continue;
            }
            i += 1;
        }

        let friction = self.s.user_friction;
        let u = self.user_mut(uid);
        if u.accel.length_squared() > u.speed * u.speed {
            u.accel = normalise_vec2(u.accel) * u.speed;
        }
        u.pos += u.accel * dt;
        u.accel *= f32_max(0.0, 1.0 - friction * dt);
    }

    /// `RAction::logic` / `RemoveAction::logic` for `active_actions[idx]` of `uid`.
    fn action_logic(&mut self, uid: UserId, idx: usize, dt: f32) {
        let a = self.user(uid).active_actions[idx].clone();
        let old_progress = a.progress;
        if a.progress >= 1.0 {
            return;
        }
        if a.progress == 0.0 {
            // apply()
            self.file_touch(a.target, a.timestamp, a.colour);
            if a.kind == Kind::Modify {
                self.file_mut(a.target).file_colour = a.modify_colour;
            }
        }
        let u = self.user(uid);
        let pending = u.action_count as f32;
        let action_rate = f32::min(10.0, a.rate * f32_max(1.0, pending));
        let progress = f32::min(a.progress + action_rate * dt, 1.0);
        self.user_mut(uid).active_actions[idx].progress = progress;

        if a.kind == Kind::Remove && old_progress < 1.0 && progress >= 1.0 {
            self.file_remove(a.target, a.timestamp);
        }
    }

    // ---- files ------------------------------------------------------------------------

    pub(crate) fn file_abs_pos(&self, f: FileId) -> Vec2 {
        let file = self.file(f);
        file.pos + self.dir(file.dir.expect("file attached to a dir")).pos
    }

    /// `RFile::touch`.
    fn file_touch(&mut self, f: FileId, touched: i64, colour: Vec3) {
        {
            let file = self.file(f);
            if file.forced_removal || (file.removing && touched < file.removed_timestamp) {
                return;
            }
        }
        let was_expired = {
            let file = self.file_mut(f);
            file.fade_start = -1.0;
            file.removing = false;
            file.removed_timestamp = 0;
            file.last_action = file.elapsed;
            file.touch_colour = colour;
            file.expired
        };

        // un expire file if touched after being removed
        if was_expired {
            if let Some(p) = self.removed.iter().position(|&x| x == f) {
                self.removed.remove(p);
            }
            self.file_mut(f).expired = false;
        }

        // showName()
        let file = self.file_mut(f);
        if file.name_interval <= 0.0 {
            file.name_interval = file.nametime;
        }
        // setHidden(false)
        let dir = file.dir;
        let was_hidden = file.hidden;
        file.hidden = false;
        if let (true, Some(d)) = (was_hidden, dir) {
            let dd = self.dir_mut(d);
            dd.visible_count += 1;
            dd.visible = true;
        }
        if let Some(d) = dir {
            self.dir_file_updated(d, true);
        }
    }

    /// `RFile::remove(timestamp)`.
    fn file_remove(&mut self, f: FileId, removed_timestamp: i64) {
        let file = self.file_mut(f);
        file.last_action = file.elapsed;
        file.fade_start = file.elapsed;
        file.removing = true;
        file.removed_timestamp = removed_timestamp;
    }

    /// `RFile::getColour`.
    pub(crate) fn file_colour(&self, f: FileId) -> Vec3 {
        let file = self.file(f);
        let lc = file.elapsed - file.last_action;
        if lc < 1.0 {
            return file.touch_colour * (1.0 - lc) + file.file_colour * lc;
        }
        file.file_colour
    }

    /// `RFile::getAlpha`.
    pub(crate) fn file_alpha(&self, f: FileId) -> f32 {
        let file = self.file(f);
        let mut alpha = f32::min(file.elapsed / 1.0, 1.0);
        if file.fade_start > 0.0 {
            alpha = (1.0f64 - (file.elapsed - file.fade_start).clamp(0.0, 1.0) as f64) as f32;
        }
        alpha
    }

    /// `RUser::getAlpha`.
    pub(crate) fn user_alpha(&self, u: UserId) -> f32 {
        let user = self.user(u);
        let mut alpha = f32::min(user.elapsed / 1.0, 1.0);
        if user.elapsed - user.last_action > self.s.user_idle_time {
            alpha = (1.0f64
                - f32::min(user.elapsed - user.last_action - self.s.user_idle_time, 1.0) as f64)
                as f32;
        }
        alpha
    }

    /// `RFile::logic`.
    fn file_logic(&mut self, f: FileId, dt: f32) {
        let file_idle_time = self.file_idle_time;
        let file = self.file_mut(f);
        // Pawn::logic
        file.elapsed += dt;
        if !file.hidden && file.name_interval > 0.0 {
            file.name_interval -= dt;
        }

        let dest_pos = file.dest * file.distance;
        file.accel = dest_pos - file.pos;

        // apply accel
        let mut accel2 = file.accel * file.speed * dt;
        if accel2.length_squared() > file.accel.length_squared() {
            accel2 = file.accel;
        }
        file.pos += accel2;

        // files have no momentum
        file.accel = Vec2::ZERO;

        if file.fade_start < 0.0
            && file_idle_time > 0.0
            && (file.elapsed - file.last_action) > file_idle_time
        {
            file.fade_start = file.elapsed;
        }

        // has completely faded out
        let mut expire = false;
        if file.fade_start > 0.0 && !file.expired && (file.elapsed - file.fade_start) >= 1.0 {
            file.expired = true;
            expire = true;
        }
        if file.hidden && !file.forced_removal {
            file.elapsed = 0.0;
        }
        if expire && !self.removed.contains(&f) {
            self.removed.push(f);
        }
    }

    // ---- directories --------------------------------------------------------------------

    /// `RDirNode::RDirNode(0, abspath)`. The C++ parent argument is always zeroed by the
    /// constructor itself, so a node starts detached at the origin.
    fn dir_new(&mut self, abspath: &str) -> DirId {
        let mut path = abspath.to_string();
        if !path.ends_with('/') {
            path.push('/');
        }
        let padded_file_radius = (FILE_DIAMETER as f64 * 0.5) as f32;
        let file_area = (padded_file_radius * padded_file_radius) as f64 * PI;
        let id = self.dirs.len();
        self.dirs.push(Some(Dir {
            abspath: path.clone(),
            parent: None,
            children: Vec::new(),
            files: Vec::new(),
            pos: Vec2::ZERO,
            spos: Vec2::ZERO,
            accel: Vec2::ZERO,
            prev_accel: Vec2::ZERO,
            dir_area: 0.0,
            file_area: file_area as f32,
            dir_radius: 0.0,
            parent_radius: 0.0,
            visible: false,
            position_initialized: false,
            visible_count: 0,
            depth: 1,
            since_node_visible: 0.0,
            since_last_file_change: 0.0,
            since_last_node_change: 0.0,
            col: [0.0; 4],
            quad: Bounds2D::default(),
        }));
        self.dir_map.insert(path, id);
        self.dir_calc_radius(id);
        self.dir_calc_colour(id);
        id
    }

    /// `~RDirNode`: children first, then the map entry (erased by key, whoever owns it).
    fn dir_delete(&mut self, d: DirId) {
        for c in self.dir(d).children.clone() {
            self.dir_delete(c);
        }
        let path = self.dir(d).abspath.clone();
        self.dir_map.remove(&path);
        self.dirs[d] = None;
    }

    fn dir_change_path(&mut self, d: DirId, abspath: &str) {
        let old = self.dir(d).abspath.clone();
        self.dir_map.remove(&old);
        let mut path = abspath.to_string();
        if !path.ends_with('/') {
            path.push('/');
        }
        self.dir_mut(d).abspath = path.clone();
        self.dir_map.insert(path, d);
    }

    fn dir_prefixed_by(&self, d: DirId, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        let abspath = &self.dir(d).abspath;
        if !path.ends_with('/') {
            abspath.starts_with(&format!("{path}/"))
        } else {
            abspath.starts_with(path)
        }
    }

    fn dir_is_dir(&self, d: DirId, path: &str) -> bool {
        if self.dir_prefixed_by(d, path) {
            return true;
        }
        if !path.starts_with(&self.dir(d).abspath) {
            return false;
        }
        self.dir(d)
            .children
            .iter()
            .any(|&c| self.dir_is_dir(c, path))
    }

    /// Directories closest to the root prefixed by `path`.
    fn dir_find_dirs(&self, d: DirId, path: &str, out: &mut Vec<DirId>) {
        if self.dir_prefixed_by(d, path) {
            out.push(d);
            return;
        }
        for &c in &self.dir(d).children {
            self.dir_find_dirs(c, path, out);
        }
    }

    /// `getFilesRecursive`: each directory's files are PREPENDED, so the result lists the
    /// last directory visited first.
    fn dir_files_recursive(&self, d: DirId, out: &mut Vec<FileId>) {
        let mut v = self.dir(d).files.clone();
        v.append(out);
        *out = v;
        for &c in &self.dir(d).children {
            self.dir_files_recursive(c, out);
        }
    }

    fn dir_set_parent(&mut self, d: DirId, parent: Option<DirId>) {
        if parent.is_some() && self.dir(d).parent == parent {
            return;
        }
        self.dir_mut(d).parent = parent;
        self.dir_adjust_depth(d);
    }

    fn dir_adjust_depth(&mut self, d: DirId) {
        let depth = match self.dir(d).parent {
            None => 1,
            Some(p) => self.dir(p).depth + 1,
        };
        self.dir_mut(d).depth = depth;
        for c in self.dir(d).children.clone() {
            self.dir_adjust_depth(c);
        }
    }

    fn dir_add_node(&mut self, d: DirId, node: DirId) {
        // does this node prefix any other nodes, if so, add them to it
        let path = self.dir(node).abspath.clone();
        for c in self.dir(d).children.clone() {
            if self.dir_prefixed_by(c, &path) {
                self.dir_mut(d).children.retain(|&x| x != c);
                self.dir_add_node(node, c);
            }
        }
        self.dir_mut(d).children.push(node);
        self.dir_set_parent(node, Some(d));
        self.dir_node_updated(d, false);
    }

    fn dir_file_updated(&mut self, d: DirId, user_initiated: bool) {
        self.dir_calc_radius(d);
        self.dir_mut(d).since_last_file_change = 0.0;
        self.dir_node_updated(d, user_initiated);
    }

    fn dir_node_updated(&mut self, d: DirId, user_initiated: bool) {
        let mut cur = d;
        let mut ui = user_initiated;
        loop {
            if ui {
                self.dir_mut(cur).since_last_node_change = 0.0;
            }
            self.dir_calc_radius(cur);
            self.dir_update_file_positions(cur);
            let dd = self.dir_mut(cur);
            if dd.visible && dd.children.is_empty() && dd.files.is_empty() {
                dd.visible = false;
            }
            match dd.parent {
                Some(p) => {
                    cur = p;
                    ui = true;
                }
                None => break,
            }
        }
    }

    /// `isVisible`: caches `true` from any visible child (a mutation the dump must observe).
    pub(crate) fn dir_is_visible(&mut self, d: DirId) -> bool {
        if self.dir(d).visible {
            return true;
        }
        for c in self.dir(d).children.clone() {
            if self.dir_is_visible(c) {
                self.dir_mut(d).visible = true;
                return true;
            }
        }
        false
    }

    fn dir_empty(&self, d: DirId) -> bool {
        let dd = self.dir(d);
        dd.visible_count == 0 && dd.children.is_empty()
    }

    fn dir_is_ancestor(&self, d: DirId, node: DirId) -> bool {
        // `isParent(node)`: node is d's parent or a further ancestor
        let mut cur = self.dir(d).parent;
        while let Some(p) = cur {
            if p == node {
                return true;
            }
            cur = self.dir(p).parent;
        }
        false
    }

    fn dir_common_path_prefix(&self, d: DirId, s: &str) -> String {
        let a = self.dir(d).abspath.as_bytes();
        let b = s.as_bytes();
        let mut c = 0;
        let mut slash: isize = -1;
        while c < a.len() && c < b.len() && a[c] == b[c] {
            if a[c] == b'/' {
                slash = c as isize;
            }
            c += 1;
        }
        if slash == -1 {
            return String::new();
        }
        s[..=(slash as usize)].to_string()
    }

    /// `RDirNode::addFile`.
    fn dir_add_file(&mut self, d: DirId, f: FileId) -> bool {
        let fpath = self.file(f).path.clone();

        // doesnt match this path at all
        if !fpath.starts_with(&self.dir(d).abspath) {
            if self.dir(d).parent.is_some() {
                return false;
            }
            // the root forks when a file's path does not match: the caller re-reads `root`
            let mut common = self.dir_common_path_prefix(d, &fpath);
            if common.is_empty() {
                common = "/".to_string();
            }
            let newparent = self.dir_new(&common);
            self.dir_add_node(newparent, d);
            return self.dir_add_file(newparent, f);
        }

        // simply change path of node and add this to it
        {
            let dd = self.dir(d);
            if dd.parent.is_none()
                && dd.abspath == "/"
                && fpath != dd.abspath
                && dd.files.is_empty()
                && dd.children.is_empty()
            {
                self.dir_change_path(d, &fpath);
            }
        }

        // is this dir - add to this node
        if fpath == self.dir(d).abspath {
            self.dir_mut(d).files.push(f);
            if !self.file(f).hidden {
                self.dir_mut(d).visible_count += 1;
            }
            self.file_mut(f).dir = Some(d);
            self.dir_file_updated(d, false);
            return true;
        }

        // does this belong to one of the children ?
        let mut added = false;
        for c in self.dir(d).children.clone() {
            added = self.dir_add_file(c, f);
            if added {
                break;
            }
        }

        if added && self.dir(d).parent.is_some() {
            return true;
        }

        // a file whose fullpath prefixes this file's path is really a directory: remove it
        // (for the root this happens even when a child took the file)
        let fpath_full = self.file(f).path.clone();
        for file in self.dir(d).files.clone() {
            if fpath_full.starts_with(&self.file(file).fullpath) {
                let fl = self.file_mut(file);
                fl.forced_removal = true;
                self.file_remove(file, 0);
                break;
            }
        }

        if added {
            return true;
        }

        // add new child, add it to that
        let node = self.dir_new(&fpath);
        self.dir_add_file(node, f);
        self.dir_add_node(d, node);

        // do we have dir nodes, with a common path element greater than abspath,
        // if so create another node, and move those nodes there
        let abspath_len = self.dir(d).abspath.len();
        let mut commonpath = String::new();
        let mut common_pos = Vec2::ZERO;
        for &c in &self.dir(d).children {
            let common = self.dir_common_path_prefix(c, &fpath);
            if common.len() > abspath_len && common != fpath {
                commonpath = common;
                common_pos = self.dir(c).pos;
                break;
            }
        }

        // redistribute to new common node
        if commonpath.len() > abspath_len {
            let cnode = self.dir_new(&commonpath);
            self.dir_mut(cnode).pos = common_pos;
            for c in self.dir(d).children.clone() {
                if self.dir_prefixed_by(c, &commonpath) {
                    self.dir_mut(d).children.retain(|&x| x != c);
                    self.dir_add_node(cnode, c);
                }
            }
            self.dir_add_node(d, cnode);
        }

        true
    }

    /// `RDirNode::removeFile` (the caller deletes the file).
    fn dir_remove_file(&mut self, d: DirId, f: FileId) -> bool {
        let fpath = self.file(f).path.clone();
        let abspath = self.dir(d).abspath.clone();

        // doesnt match this path at all
        if !fpath.starts_with(&abspath) {
            return false;
        }

        // is this dir
        if fpath == abspath {
            if let Some(i) = self.dir(d).files.iter().position(|&x| x == f) {
                self.dir_mut(d).files.remove(i);
                if !self.file(f).hidden {
                    self.dir_mut(d).visible_count -= 1;
                }
                self.dir_file_updated(d, false);
                return true;
            }
            return false;
        }

        // does this belong to one of the children ?
        for (i, c) in self.dir(d).children.clone().into_iter().enumerate() {
            if self.dir_remove_file(c, f) {
                // node is now empty, reap!
                if self.dir(c).files.is_empty() && self.dir(c).children.is_empty() {
                    self.dir_mut(d).children.remove(i);
                    self.dir_delete(c);
                    self.dir_node_updated(d, false);
                }
                return true;
            }
        }
        false
    }

    /// `RDirNode::calcRadius`. Reads the children's `dir_area` as last stored (stale if the
    /// child has not been recomputed yet); that ordering is part of the contract.
    fn dir_calc_radius(&mut self, d: DirId) {
        let dd = self.dir(d);
        let total_file_area = dd.file_area * dd.visible_count as f32;
        let mut dir_area = total_file_area;
        for &c in &dd.children {
            dir_area += self.dir(c).dir_area;
        }
        let dd = self.dir_mut(d);
        dd.dir_area = dir_area;
        dd.dir_radius = f32_max(1.0, (dir_area as f64).sqrt() as f32) * DIR_PADDING;
        dd.parent_radius = f32_max(1.0, (total_file_area as f64).sqrt() as f32 * DIR_PADDING);
    }

    /// `RDirNode::calcColour`.
    fn dir_calc_colour(&mut self, d: DirId) {
        let since = self.dir(d).since_last_node_change;
        let brightness = f32_max(0.6, 1.0 - f32::min(1.0, since / 3.0));
        let mut col = [brightness, brightness, brightness, 1.0f32];
        let mut fcount = 0;
        for &f in &self.dir(d).files {
            if self.file(f).hidden {
                continue;
            }
            let filecol = self.file_colour(f) * brightness;
            let a = self.file_alpha(f);
            col[0] += filecol.x;
            col[1] += filecol.y;
            col[2] += filecol.z;
            col[3] += a;
            fcount += 1;
        }
        let div = (fcount as f32 as f64 + 1.0) as f32;
        for c in &mut col {
            *c /= div;
        }
        self.dir_mut(d).col = col;
    }

    /// `RDirNode::updateFilePositions`.
    fn dir_update_file_positions(&mut self, d: DirId) {
        let mut max_files: i32 = 1;
        let mut diameter: i32 = 1;
        let mut file_no: i32 = 0;
        let mut dist: f32 = 0.0;
        let mut files_left = self.dir(d).visible_count;

        for f in self.dir(d).files.clone() {
            if self.file(f).hidden {
                let fl = self.file_mut(f);
                fl.dest = Vec2::ZERO;
                fl.distance = 0.0;
                continue;
            }

            // calcFileDest: `1.0/(float)max_files` and the angle are double expressions
            let arc = (1.0f64 / (max_files as f32) as f64) as f32;
            // `arc * file_no` is float * int, so it is a float product; only `arc * 0.5` is double
            let frac = (arc as f64 * 0.5 + (arc * file_no as f32) as f64) as f32;
            let angle = (frac as f64 * PI * 2.0) as f32;
            let dest = vec2(angle.sin(), angle.cos());

            let fl = self.file_mut(f);
            fl.dest = dest;
            fl.distance = dist;

            files_left -= 1;
            file_no += 1;

            if file_no >= max_files {
                diameter += 1;
                dist += FILE_DIAMETER;
                max_files = f64::max(1.0, diameter as f64 * PI) as i32;

                if files_left < max_files {
                    max_files = files_left;
                }
                file_no = 0;
            }
        }
    }

    fn dir_distance_to_parent(&self, d: DirId) -> f32 {
        let dd = self.dir(d);
        let parent = self.dir(dd.parent.expect("has parent"));
        let posd = (parent.pos - dd.pos).length();
        posd - (dd.dir_radius + parent.parent_radius)
    }

    /// `RDirNode::applyForceDir(node)` on `d`.
    fn dir_apply_force_dir(&mut self, d: DirId, node: DirId) {
        if node == d {
            return;
        }
        let me = self.dir(d);
        let other = self.dir(node);
        let dir = other.pos - me.pos;

        let posd2 = dir.length_squared();
        let myradius = me.dir_radius;
        let your_radius = other.dir_radius;

        let sumradius = myradius + your_radius;
        let distance2 = posd2 - sumradius * sumradius;

        if (distance2 as f64) > 0.0 {
            return;
        }

        let posd = posd2.sqrt();
        let distance = posd - myradius - your_radius;

        // resolve overlap
        if (posd as f64) < 0.00001 {
            let rx = self.rand() % 100;
            let ry = self.rand() % 100;
            let v = normalise_vec2(vec2((rx - 50) as f32, (ry - 50) as f32));
            self.dir_mut(d).accel += v;
            return;
        }

        self.dir_mut(d).accel += distance * normalise_vec2(dir);
    }

    /// `RDirNode::applyForces`: children first, then this node.
    fn dir_apply_forces(&mut self, d: DirId, tree: &QuadTree<DirId>) {
        for c in self.dir(d).children.clone() {
            self.dir_apply_forces(c, tree);
        }

        let Some(parent) = self.dir(d).parent else {
            return;
        };

        // DirForceFunctor. Quadtree visit order decides the f32 accumulation order.
        let quad = self.dir(d).quad;
        let mut seen: Vec<DirId> = Vec::new();
        let mut visit: Vec<DirId> = Vec::new();
        tree.visit_items_in_bounds(&quad, |n, node_count| {
            if n == d || n == parent || self.dir(n).parent == Some(d) {
                return;
            }
            if self.dir_is_ancestor(d, n) || self.dir_is_ancestor(n, d) {
                return;
            }
            if node_count != 1 {
                if seen.contains(&n) {
                    return;
                }
                seen.push(n);
            }
            visit.push(n);
        });
        for n in visit {
            self.dir_apply_force_dir(d, n);
        }

        // always call on parent no matter how far away
        self.dir_apply_force_dir(d, parent);

        // pull towards parent
        let parent_dist = self.dir_distance_to_parent(d);
        let ppos = self.dir(parent).pos;
        let pos = self.dir(d).pos;
        let gravity = self.force_gravity;
        self.dir_mut(d).accel += gravity * parent_dist * normalise_vec2(ppos - pos);

        // dirs should be pushed along the parent_parent to parent normal by a force smaller
        // than the parent radius force
        if let Some(pparent) = self.dir(parent).parent {
            let parent_edge = ppos - self.dir(pparent).pos;
            let parent_edge_normal = normalise_vec2(parent_edge);
            let dest = (ppos
                + (self.dir(parent).dir_radius + self.dir(d).dir_radius) * parent_edge_normal)
                - pos;
            self.dir_mut(d).accel += dest;
        }

        // dirs should repulse from other dirs of this parent
        let siblings = self.dir(parent).children.clone();
        if !siblings.is_empty() {
            let mut sib_accel = Vec2::ZERO;
            let mut visible = 1;
            for n in siblings {
                if n == d {
                    continue;
                }
                if !self.dir_is_visible(n) {
                    continue;
                }
                visible += 1;
                sib_accel -= normalise_vec2(self.dir(n).pos - pos);
            }

            // parent circumference divided by the number of visible child nodes
            if visible > 1 {
                // `float slice_size = (radius * PI) / (float)(visible+1)`: the division is double
                let slice_size = ((self.dir(parent).dir_radius as f64 * PI)
                    / ((visible + 1) as f32 as f64)) as f32;
                sib_accel *= slice_size;
                self.dir_mut(d).accel += sib_accel;
            }
        }
    }

    /// `RDirNode::setInitialPosition`.
    fn dir_set_initial_position(&mut self, d: DirId) {
        let parent = self.dir(d).parent.expect("has parent");
        let parent_pos = self.dir(parent).pos;
        let abspath = self.dir(d).abspath.clone();
        let h = Vec2::from_array(vec2_hash(abspath.as_bytes(), self.s.hash_seed));
        let mut pos = parent_pos;
        if let Some(pp) = self.dir(parent).parent {
            // pos += normalise(normalise(parent.pos - parentP.pos) * 2.0f + vec2Hash(abspath))
            let pp_pos = self.dir(pp).pos;
            pos += normalise_vec2(normalise_vec2(parent_pos - pp_pos) * 2.0 + h);
        } else {
            pos += h;
        }
        let dd = self.dir_mut(d);
        dd.pos = pos;
        // the spline point
        dd.spos = pos - (parent_pos - pos) * 0.5;
        dd.position_initialized = true;
    }

    /// `RDirNode::logic`.
    fn dir_logic(&mut self, d: DirId, dt: f32) {
        // move
        if let Some(parent) = self.dir(d).parent {
            // initial position
            if !self.dir_empty(d) && !self.dir(d).position_initialized {
                self.dir_set_initial_position(d);
            }

            let elasticity = self.s.elasticity;
            let dd = self.dir_mut(d);
            dd.pos += dd.accel * dt;

            if (elasticity as f64) > 0.0 {
                let diff = dd.accel - dd.prev_accel;
                let m = dt * elasticity;
                let accel3 = dd.prev_accel * (1.0 - m) + diff * m;
                dd.pos += accel3;
                dd.prev_accel = accel3;
            }
            dd.accel = Vec2::ZERO;

            // updateSplinePoint
            let ppos = self.dir(parent).pos;
            let dd = self.dir_mut(d);
            let td = (ppos - dd.pos) * 0.5;
            let mid = dd.pos + td;
            let delta = mid - dd.spos;
            // dont let spos get more than half the length of the distance behind
            if delta.length_squared() > td.length_squared() {
                dd.spos += normalise_vec2(delta) * (delta.length() - td.length());
            }
            dd.spos += delta * f32::min(1.0, dt * 2.0);
        }

        // update files
        for f in self.dir(d).files.clone() {
            self.file_logic(f, dt);
        }

        // update child nodes
        for c in self.dir(d).children.clone() {
            self.dir_logic(c, dt);
        }

        // update colour
        self.dir_calc_colour(d);

        // update tickers
        let dd = self.dir_mut(d);
        if dd.visible {
            dd.since_node_visible += dt;
        }
        dd.since_last_file_change += dt;
        dd.since_last_node_change += dt;
    }

    fn interact_dirs(&mut self) -> QuadTree<DirId> {
        let mut b = self.dir_bounds;
        b.min -= vec2(1.0, 1.0);
        b.max += vec2(1.0, 1.0);

        let max_depth = if self.dir_bounds.area() > 10000.0 {
            MAX_QUADTREE_DEPTH
        } else {
            1
        };
        let mut tree: QuadTree<DirId> = QuadTree::new(b, max_depth, 1);

        // gGourceDirMap order is the insertion order the force functor later sees
        for &d in self.dir_map.values() {
            if !self.dir_empty(d) {
                tree.add_item(d, self.dir(d).quad);
            }
        }
        tree
    }

    fn update_dirs(&mut self, dt: f32, tree: &QuadTree<DirId>) {
        let root = self.root;
        self.dir_apply_forces(root, tree);
        self.dir_logic(root, dt);
    }

    fn update_camera(&mut self, dt: f32) {
        let mut auto_rotate = !self.manual_rotate && !self.s.disable_auto_rotate;

        if self.manual_camera {
            if self.cursor_move.length_squared() > 0.0 {
                let cam_rate = (-self.camera.pos.z) / 5000.0f32;
                let mut cam_pos = self.camera.pos;
                let delta = self.cursor_move * cam_rate * 10.0f32;
                cam_pos.x += delta.x;
                cam_pos.y += delta.y;
                self.camera.set_pos_keep_angle(cam_pos);
                self.camera.stop();
                auto_rotate = false;
                self.cursor_move = Vec2::ZERO;
            }
        } else {
            let cambounds = if self.track_users
                && (self.selected_file.is_some() || self.selected_user.is_some())
            {
                // track mode on a selection: just that user and/or file
                let mut b = Bounds2D::default();
                if let Some(uid) = self.selected_user {
                    b.update(self.user(uid).pos);
                }
                if let Some(f) = self.selected_file {
                    b.update(self.file_abs_pos(f));
                }
                b
            } else if self.track_users && self.idle_time == 0.0 {
                self.active_user_bounds
            } else {
                self.dir_bounds
            };
            self.camera.adjust(&cambounds, &self.s, !self.manual_zoom);
        }
        self.camera.logic(dt);

        // automatically rotate camera
        if !auto_rotate {
            self.rotation_remaining_angle = 0.0;
        } else if self.rotation_remaining_angle > 0.0 {
            // rotation through 90 degrees, speed peaks at half way. `fabs(x - 0.5)` is a double
            // (`0.5` literal), so is the product with `2.0f`; the cast narrows it back.
            let remaining = self.rotation_remaining_angle;
            let peak = (1.0f32 as f64 - (((remaining / 90.0f32) as f64) - 0.5).abs() * 2.0) as f32;
            let angle_rate = f32_max(dt, peak) * dt;

            self.rotate_angle = f32::min(remaining, 90.0f32 * angle_rate);
            self.rotation_remaining_angle -= self.rotate_angle;

            // `float *= double`: the product is a double, narrowed on assignment
            self.rotate_angle =
                (self.rotate_angle as f64 * burl_util::math::DEGREES_TO_RADIANS) as f32;
        } else if !self.right_button && self.dir_bounds.area() > 10000.0f32 {
            let aspect_ratio = self.s.display_width as f32 / self.s.display_height as f32;
            let b = &self.dir_bounds;
            let bounds_ratio = if aspect_ratio > 1.0 {
                b.width() / b.height()
            } else {
                b.height() / b.width()
            };
            if bounds_ratio < 0.67f32 {
                self.rotation_remaining_angle = 90.0;
            }
        }
    }

    /// The head of `Gource::logic`: apply last frame's `rotate_angle` to the whole tree.
    fn apply_rotation(&mut self) {
        if self.rotate_angle == 0.0 {
            return;
        }
        let s = self.rotate_angle.sin();
        let c = self.rotate_angle.cos();
        let root = self.root;
        // a manual (right-drag) rotation turns the tree about the camera, an automatic one
        // about the origin
        let centre = self
            .manual_rotate
            .then(|| vec2(self.camera.pos.x, self.camera.pos.y));
        self.dir_rotate(root, s, c, centre);
        for &uid in self.user_map.values().copied().collect::<Vec<_>>().iter() {
            let u = self.user_mut(uid);
            u.pos = rotate_about(u.pos, s, c, centre);
        }
        self.rotate_angle = 0.0;
    }

    /// `RDirNode::rotate(s, c)` / `rotate(s, c, centre)`.
    fn dir_rotate(&mut self, d: DirId, s: f32, c: f32, centre: Option<Vec2>) {
        let dir = self.dir_mut(d);
        dir.pos = rotate_about(dir.pos, s, c, centre);
        dir.spos = rotate_about(dir.spos, s, c, centre);
        for child in self.dir(d).children.clone() {
            self.dir_rotate(child, s, c, centre);
        }
    }

    /// `Gource::loadCaptions`, called once when the clock starts.
    fn load_captions(&mut self) {
        if self.s.caption_file.is_empty() {
            return;
        }
        let Ok(bytes) = std::fs::read(&self.s.caption_file) else {
            return;
        };
        let mut last_timestamp = 0i64;
        for line in bytes.split(|&b| b == b'\n') {
            // ConfFile::trim: trailing whitespace only
            let end = line
                .iter()
                .rposition(|b| !b" \t\x0c\x0b\n\r".contains(b))
                .map_or(0, |i| i + 1);
            let Some((stamp, text)) = split_caption(&line[..end]) else {
                continue;
            };
            let timestamp = if stamp.len() > 1 && stamp[1..].contains(&b'-') {
                match burl_util::timeparse::parse_date_time(stamp) {
                    Some(t) => t,
                    None => continue,
                }
            } else {
                let t = atoll(stamp);
                if t == 0 && stamp != b"0" {
                    continue;
                }
                t
            };
            // ignore older captions, then out of order ones
            if timestamp < self.currtime || timestamp < last_timestamp {
                continue;
            }
            last_timestamp = timestamp;
            self.captions.push_back(Caption {
                text: burl_util::utf8::filter_utf8(text),
                timestamp,
                elapsed: 0.0,
                alpha: 0.0,
            });
        }
    }

    /// The caption part of `Gource::logic`: queued captions whose time has come become active;
    /// active ones age and expire. (Their on-screen row, `setPos`, is renderer-only.)
    fn update_captions(&mut self, dt: f32) {
        while self
            .captions
            .front()
            .is_some_and(|c| c.timestamp <= self.currtime)
        {
            let c = self.captions.pop_front().unwrap();
            self.active_captions.push(c);
            self.captions_activated += 1;
        }
        let duration = self.s.caption_duration;
        self.active_captions.retain_mut(|c| {
            // RCaption::logic, then isFinished
            let fade_in = f32::min(2.0, duration / 3.0);
            c.elapsed += dt;
            c.alpha = f32::min(
                1.0,
                f32::min(c.elapsed, f32::max(0.0, duration - c.elapsed)) / fade_in,
            );
            c.elapsed < duration
        });
    }
}

/// `^(?:\xEF\xBB\xBF)?([^|]+)\|(.+)$` on one line: the timestamp and the text. The optional BOM
/// is tried first, then (as the regex backtracks) not at all.
fn split_caption(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let split = |s: &'_ [u8]| -> Option<(usize, usize)> {
        let bar = s.iter().position(|&b| b == b'|')?;
        (bar > 0 && bar + 1 < s.len() && !s[bar + 1..].contains(&b'\n')).then_some((bar, bar + 1))
    };
    let body = line.strip_prefix(b"\xEF\xBB\xBF".as_slice());
    for s in body.into_iter().chain(Some(line)) {
        if let Some((bar, rest)) = split(s) {
            return Some((&s[..bar], &s[rest..]));
        }
    }
    None
}

/// C `atoll`: leading whitespace, optional sign, digits; saturates like `strtoll`.
fn atoll(s: &[u8]) -> i64 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let neg = s.get(i) == Some(&b'-');
    if matches!(s.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let mut v: i128 = 0;
    while let Some(d) = s.get(i).filter(|b| b.is_ascii_digit()) {
        v = (v * 10 + (d - b'0') as i128).min(1 << 70);
        i += 1;
    }
    let v = if neg { -v } else { v };
    v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

impl Sim {
    pub(crate) fn commitqueue_len(&self) -> usize {
        self.commitqueue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;
    use burl_formats::{fetch_log, LogSettings};
    use std::sync::Arc;

    fn sim(settings: SimSettings) -> Result<Sim, SimError> {
        sim_with("1277787455|bob|A|a/b.txt\n", settings)
    }

    fn sim_with(text: &str, settings: SimSettings) -> Result<Sim, SimError> {
        let path = std::env::temp_dir().join(format!(
            "burl-sim-unit-{}-{:?}.log",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&path, text).unwrap();
        let log = fetch_log(
            path.to_str().unwrap(),
            Arc::new(LogSettings {
                log_format: "custom".into(),
                ..LogSettings::default()
            }),
        )
        .unwrap();
        let _ = std::fs::remove_file(&path);
        Sim::new(settings, log, SimRng::from_script(vec![1, 2, 3]).unwrap())
    }

    fn ok_settings() -> SimSettings {
        SimSettings {
            disable_auto_rotate: true,
            ..SimSettings::default()
        }
    }

    fn run(s: &mut Sim, frames: usize) {
        for _ in 0..frames {
            s.update().unwrap();
            s.end_frame();
        }
    }

    const THREE: &str =
        "1277787455|bob|A|a/b.txt\n1277787456|bob|A|c/d.txt\n1277787457|bob|A|c/e/f.txt\n";

    /// `--start-position random`: `rand() % 1000 / 1000.0f` is drawn at import (the script's first
    /// value is 1), counts as a `rand()` call and sets `first_rand_frame` to 0.
    #[test]
    fn start_position_random_draws_from_the_prng() {
        let s = sim_with(
            THREE,
            SimSettings {
                start_position_random: true,
                ..ok_settings()
            },
        )
        .unwrap();
        assert_eq!(s.s.start_position, 0.001);
        assert_eq!((s.rand_calls(), s.first_rand_frame), (1, 0));
    }

    /// The slider: `commit_time_at` peeks at the commit under a percent without moving the
    /// read position, and `SeekTo` (a click) restarts the log there and ends a pause.
    #[test]
    fn slider_peek_and_seek() {
        let mut s = sim_with(THREE, ok_settings()).unwrap();
        let first = s.commit_time_at(0.0).unwrap();
        let late = s.commit_time_at(0.5).unwrap();
        assert!(late > first);
        assert_eq!(s.commit_time_at(1.0), None);
        // peeking left the log where it was: the next commit is still the first
        assert_eq!(s.commit_time_at(0.0), Some(first));
        s.update().unwrap();
        assert_eq!(s.scene_view().time, first);
        s.apply(Command::TogglePause);
        s.apply(Command::SeekTo(0.5));
        assert!(!s.paused);
        s.update().unwrap();
        assert_eq!(s.scene_view().time, late);
    }

    /// `Gource::setFrameExporter`: at 25 fps the tick rate is 75/s and every third tick is dumped.
    #[test]
    fn exports_every_frameskip_plus_one_tick() {
        let exported = |fps| {
            let mut s = sim(SimSettings {
                output_framerate: fps,
                ..ok_settings()
            })
            .unwrap();
            (0..7)
                .map(|_| {
                    s.update().unwrap();
                    let e = s.exports_frame();
                    s.end_frame();
                    e
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(exported(25), [true, false, false, true, false, false, true]);
        assert_eq!(exported(30), [true, false, true, false, true, false, true]);
        assert_eq!(exported(60), [true; 7]);
    }

    #[test]
    fn paused_freezes_the_clock_but_not_the_camera() {
        let mut s = sim_with(THREE, ok_settings()).unwrap();
        run(&mut s, 10);
        s.apply(Command::TogglePause);
        let (runtime, currtime, files) = (s.runtime, s.currtime, s.file_map.len());
        // a zoom moves dest.z (and stops the fit), which only camera.logic can follow
        s.apply(Command::ZoomOut);
        let z = s.camera.pos.z;
        run(&mut s, 5);
        assert_eq!(
            (s.runtime, s.currtime, s.file_map.len()),
            (runtime, currtime, files)
        );
        assert_ne!(s.camera.pos.z, z);
        s.apply(Command::TogglePause);
        run(&mut s, 5);
        assert!(s.runtime > runtime);
    }

    #[test]
    fn arrows_assign_in_cursor_order_and_make_the_camera_manual() {
        let mut s = sim(ok_settings()).unwrap();
        s.apply(Command::Arrows {
            right: true,
            left: true,
            up: true,
            down: true,
        });
        assert_eq!(s.cursor_move, vec2(-10.0, 10.0));
        assert!(s.manual_camera);
        // nothing held leaves a drag's accumulated move alone
        s.apply(Command::Drag { dx: 1.0, dy: 2.0 });
        s.apply(Command::Arrows {
            right: false,
            left: false,
            up: false,
            down: false,
        });
        assert_eq!(s.cursor_move, vec2(-9.0, 12.0));
    }

    #[test]
    fn manual_camera_pans_by_cam_rate_and_stays_put() {
        let mut s = sim(ok_settings()).unwrap();
        s.camera.pos.z = -500.0;
        s.camera.dest = s.camera.pos;
        let p = s.camera.pos;
        s.apply(Command::DragStart);
        s.apply(Command::Drag { dx: 5.0, dy: -2.0 });
        s.update().unwrap();
        // cam_rate = 500/5000, times 10
        assert!((s.camera.pos.x - (p.x + 5.0)).abs() < 1e-4);
        assert!((s.camera.pos.y - (p.y - 2.0)).abs() < 1e-4);
        assert_eq!(s.cursor_move, Vec2::ZERO);
        // and the overview fit no longer drags it back
        let q = s.camera.pos;
        run(&mut s, 5);
        assert_eq!(s.camera.pos, q);
    }

    #[test]
    fn manual_rotation_turns_the_tree_about_the_camera() {
        let mut s = sim_with(THREE, ok_settings()).unwrap();
        run(&mut s, 200);
        s.camera.pos.x = 3.0;
        s.camera.pos.y = 4.0;
        let centre = vec2(3.0, 4.0);
        let dist = |s: &Sim| -> Vec<f32> {
            s.dirs
                .iter()
                .flatten()
                .map(|d| (d.pos - centre).length())
                .collect()
        };
        let before = dist(&s);
        let pos: Vec<Vec2> = s.dirs.iter().flatten().map(|d| d.pos).collect();
        s.apply(Command::Rotate { dx: 10.0, dy: 0.0 });
        s.apply_rotation();
        let after = dist(&s);
        assert!(before.iter().zip(&after).all(|(a, b)| (a - b).abs() < 1e-3));
        let moved: Vec<Vec2> = s.dirs.iter().flatten().map(|d| d.pos).collect();
        assert_ne!(pos, moved);
        assert_eq!(s.rotate_angle, 0.0);
    }

    #[test]
    fn force_gravity_scales_the_pull_toward_the_parent() {
        let go = |g: f32| {
            let mut s = sim_with(THREE, ok_settings()).unwrap();
            s.force_gravity = g;
            run(&mut s, 120);
            s.dirs.iter().flatten().map(|d| d.pos).collect::<Vec<_>>()
        };
        assert_ne!(go(10.0), go(0.0));
        let mut s = sim(ok_settings()).unwrap();
        s.apply(Command::GravityUp);
        assert!(s.force_gravity > FORCE_GRAVITY);
        s.apply(Command::GravityDown);
        assert!((s.force_gravity - FORCE_GRAVITY).abs() < 1e-5);
    }

    #[test]
    fn skip_idle_sets_the_idle_clock() {
        let mut s = sim(SimSettings {
            auto_skip_seconds: 3.0,
            ..ok_settings()
        })
        .unwrap();
        s.apply(Command::SkipIdle);
        assert_eq!(s.idle_time, 3.0);
    }

    #[test]
    fn caption_lines_split_like_the_regex() {
        assert_eq!(split_caption(b"12|a|b"), Some((&b"12"[..], &b"a|b"[..])));
        assert_eq!(
            split_caption(b"\xEF\xBB\xBF12|x"),
            Some((&b"12"[..], &b"x"[..]))
        );
        // BOM then nothing before the bar: the regex backtracks and the BOM becomes the stamp
        assert_eq!(
            split_caption(b"\xEF\xBB\xBF|x"),
            Some((&b"\xEF\xBB\xBF"[..], &b"x"[..]))
        );
        assert_eq!(split_caption(b"|x"), None);
        assert_eq!(split_caption(b"12|"), None);
        assert_eq!(split_caption(b"no bar"), None);
    }

    #[test]
    fn atoll_is_c_atoll() {
        assert_eq!(atoll(b" -12x"), -12);
        assert_eq!(atoll(b"+7"), 7);
        assert_eq!(atoll(b"abc"), 0);
        assert_eq!(atoll(b"99999999999999999999999"), i64::MAX);
    }

    #[test]
    fn captions_activate_age_and_expire() {
        let mut s = sim(SimSettings {
            caption_duration: 3.0,
            ..ok_settings()
        })
        .unwrap();
        s.currtime = 100;
        for t in [100, 200] {
            s.captions.push_back(Caption {
                text: "c".into(),
                timestamp: t,
                elapsed: 0.0,
                alpha: 0.0,
            });
        }
        s.update_captions(1.0);
        assert_eq!((s.captions.len(), s.active_captions.len()), (1, 1));
        assert_eq!(s.active_captions[0].alpha, 1.0 / 1.0f32.min(1.0));
        s.update_captions(1.0);
        s.update_captions(1.0); // elapsed 3 >= duration
        assert!(s.active_captions.is_empty());
    }

    #[test]
    fn one_commit_makes_a_user_a_file_and_its_dirs() {
        let mut s = sim(ok_settings()).unwrap();
        s.update().unwrap();
        assert_eq!((s.user_map.len(), s.file_map.len()), (1, 1));
        // the root forked onto the file's directory path
        assert!(s.dir_map.contains_key("/a/"));
        // the clock starts at the first commit and has advanced by one tick
        assert!(s.currtime >= 1277787455 && s.currtime < 1277787455 + 1000);
    }

    #[test]
    fn scene_view_walks_dirs_depth_first_with_the_file_and_user() {
        let mut s = sim_with(
            "1277787455|bob|A|a/b.txt\n1277787456|bob|A|c/d.txt\n1277787457|bob|A|c/e/f.txt\n",
            ok_settings(),
        )
        .unwrap();
        for _ in 0..600 {
            s.update().unwrap();
            s.end_frame();
        }
        let v = s.scene_view();
        // The root is first and parentless; every other dir points at an earlier one.
        assert_eq!(v.dirs[0].parent, None);
        assert!(v
            .dirs
            .iter()
            .enumerate()
            .skip(1)
            .all(|(i, d)| d.parent.is_some_and(|p| p < i)));
        assert!(v.dirs.len() > 1 && v.dirs.iter().any(|d| d.pos != Vec2::ZERO));
        // A file's absolute position is its offset plus its directory's position.
        let moved = v
            .dirs
            .iter()
            .find(|d| d.pos != Vec2::ZERO && !d.files.is_empty());
        let f = &moved.expect("a moved dir holds a file").files[0];
        assert!(s.file_map.values().any(|&id| s.file_abs_pos(id) == f.pos));
        assert!(f.alpha > 0.0 && f.alpha <= 1.0);
        assert_eq!(v.users.len(), 1);
        assert_eq!(v.camera.max_distance, s.s.camera_zoom_max);
    }

    #[test]
    fn avatar_hook_sets_ratio_bounds_colour_and_texture() {
        let run = |hook: Option<crate::UserGraphicsHook>| {
            let mut st = ok_settings();
            st.user_graphics = hook;
            let mut s = sim(st).unwrap();
            for _ in 0..30 {
                s.update().unwrap();
                s.end_frame();
            }
            let v = s.scene_view();
            (v.users[0].clone(), s.user_bounds)
        };
        let (stock, stock_bounds) = run(None);
        assert_eq!(
            (stock.graphic, stock.graphic_ratio),
            (None, USER_GRAPHIC_RATIO)
        );
        let hook = |white| {
            Some(crate::UserGraphicsHook(Arc::new(move |_| {
                Some(UserGraphic {
                    ratio: 1.0,
                    texture: Some(7),
                    white,
                })
            })))
        };
        let (white, white_bounds) = run(hook(true));
        assert_eq!((white.graphic, white.graphic_ratio), (Some(7), 1.0));
        assert_eq!(white.colour, Vec3::splat(0.9));
        // a squarer sprite is shorter, so the user's bounds shrink in y
        assert!(white_bounds.max.y - white_bounds.min.y < stock_bounds.max.y - stock_bounds.min.y);
        let (coloured, _) = run(hook(false));
        assert_eq!(coloured.colour, stock.colour);
    }
}
