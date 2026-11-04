//! A read-only snapshot of what the renderer draws (phase 5, ticket #19): plain data in the order
//! `Gource::draw` walks it, so the renderer needs no access to the simulation's arenas.
//! Ported from the draw-side accessors: `RFile::getColour/getAlpha/getAbsolutePos`,
//! `RUser::getColour/getAlpha`, `Pawn::getAlpha`, `RAction::drawToVBO` inputs and
//! `RDirNode::isVisible` (src/file.cpp, user.cpp, pawn.h, action.cpp, dirnode.cpp).

use crate::sim::{DirId, Sim};
use glam::{Vec2, Vec3};

/// `Pawn::fadetime`.
const FADE_TIME: f32 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct CameraView {
    pub pos: Vec3,
    pub target: Vec3,
    /// `ZoomCamera::max_distance` (`zfar` is this plus one).
    pub max_distance: f32,
}

/// One directory node. `parent` indexes `SceneView::dirs`, which is in depth-first order from the
/// root with children in list order, the order `calcEdges`/`updateEdgeVBO`/`updateBloomVBO` visit.
#[derive(Clone, Debug, PartialEq)]
pub struct DirView {
    pub parent: Option<usize>,
    pub pos: Vec2,
    pub spos: Vec2,
    pub colour: [f32; 4],
    pub radius: f32,
    /// `isVisible()` (the node or any descendant is visible).
    pub visible: bool,
    pub files: Vec<FileView>,
    /// `path_token`: the last path component, the directory label.
    pub name: String,
    pub depth: i32,
    pub since_last_node_change: f32,
}

/// A file that is not hidden; `pos` is absolute (`getAbsolutePos`).
#[derive(Clone, Debug, PartialEq)]
pub struct FileView {
    pub pos: Vec2,
    pub size: f32,
    pub colour: Vec3,
    pub alpha: f32,
    /// `RFile::name`, the basename.
    pub name: String,
    /// `Pawn::nametime` / `name_interval`: the label's fade clock.
    pub nametime: f32,
    pub name_interval: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionView {
    pub src: Vec2,
    pub dest: Vec2,
    pub target_size: f32,
    pub progress: f32,
    pub colour: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserView {
    pub pos: Vec2,
    pub size: f32,
    pub colour: Vec3,
    pub alpha: f32,
    pub actions: Vec<ActionView>,
    pub name: String,
    pub nametime: f32,
    pub name_interval: f32,
    pub highlighted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneView {
    pub camera: CameraView,
    pub dirs: Vec<DirView>,
    /// In name order (`std::map<std::string, RUser*>`).
    pub users: Vec<UserView>,
    /// `Gource::updateTime`'s argument: `currtime`, or `lasttime` once the queue is empty.
    pub time: i64,
    /// `commitlog->isSeekable()` (`canSeek` also needs `--hide progress` off).
    pub seekable: bool,
    /// `active_captions`, oldest first, with how many captions have ever been activated.
    pub captions: Vec<CaptionView>,
    pub captions_activated: usize,
    /// The `file_key` calls since the last snapshot, in order.
    pub key_events: Vec<KeyEvent>,
}

/// `FileKey::logic/inc/dec` as the sim made them.
#[derive(Clone, Debug, PartialEq)]
pub enum KeyEvent {
    Tick(f32),
    Inc {
        path: String,
        colour: Vec3,
    },
    Dec {
        path: String,
    },
    /// `FileKey::clear` (a seek resets the sim).
    Clear,
}

/// An active `RCaption`: its text and current opacity.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionView {
    pub text: String,
    pub alpha: f32,
}

impl Sim {
    /// Snapshot the scene after `update`. `&mut` because `isVisible` caches `visible = true` on
    /// ancestors of visible nodes, and the next tick's logic reads that flag, so drawing a frame
    /// is observable by the simulation exactly as in C++.
    pub fn scene_view(&mut self) -> SceneView {
        let mut order = Vec::new();
        let mut stack = vec![(self.root, None::<usize>)];
        // Depth-first, children in list order: push them reversed so the first child pops first.
        while let Some((d, parent)) = stack.pop() {
            let idx = order.len();
            order.push((d, parent));
            for &c in self.dir(d).children.iter().rev() {
                stack.push((c, Some(idx)));
            }
        }
        let dirs = order
            .into_iter()
            .map(|(d, parent)| self.dir_view(d, parent))
            .collect();

        let users = self
            .user_map
            .values()
            .map(|&u| {
                let u = self.user(u);
                let idle = u.elapsed - u.last_action;
                let mut alpha = (u.elapsed / FADE_TIME).min(1.0);
                if idle > self.s.user_idle_time {
                    alpha = 1.0 - (idle - self.s.user_idle_time).min(1.0);
                }
                UserView {
                    pos: u.pos,
                    size: u.size,
                    colour: u.usercol,
                    alpha,
                    name: u.name.clone(),
                    nametime: u.nametime,
                    name_interval: u.name_interval,
                    highlighted: u.highlighted,
                    actions: u
                        .active_actions
                        .iter()
                        .map(|a| {
                            let t = self.file(a.target);
                            ActionView {
                                src: u.pos,
                                dest: self.file_abs_pos(a.target),
                                target_size: t.size,
                                progress: a.progress,
                                colour: a.colour,
                            }
                        })
                        .collect(),
                }
            })
            .collect();

        SceneView {
            camera: CameraView {
                pos: self.camera.pos,
                target: self.camera.target,
                max_distance: self.s.camera_zoom_max,
            },
            dirs,
            users,
            time: if self.commitqueue.is_empty() {
                self.lasttime
            } else {
                self.currtime
            },
            seekable: self.log.is_seekable(),
            captions: self
                .active_captions
                .iter()
                .map(|c| CaptionView {
                    text: c.text.clone(),
                    alpha: c.alpha,
                })
                .collect(),
            captions_activated: self.captions_activated,
            key_events: std::mem::take(&mut self.key_events),
        }
    }

    fn dir_view(&mut self, d: DirId, parent: Option<usize>) -> DirView {
        let visible = self.dir_is_visible(d);
        let dd = self.dir(d);
        let files = dd
            .files
            .iter()
            .map(|&f| self.file(f))
            .filter(|f| !f.hidden)
            .map(|f| {
                let lc = f.elapsed - f.last_action;
                let colour = if lc < 1.0 {
                    f.touch_colour * (1.0 - lc) + f.file_colour * lc
                } else {
                    f.file_colour
                };
                let mut alpha = (f.elapsed / FADE_TIME).min(1.0);
                if f.fade_start > 0.0 {
                    alpha = 1.0 - (f.elapsed - f.fade_start).clamp(0.0, 1.0);
                }
                let name = f.fullpath.rsplit('/').next().unwrap_or_default();
                FileView {
                    pos: f.pos + dd.pos,
                    size: f.size,
                    colour,
                    alpha,
                    name: name.to_string(),
                    nametime: f.nametime,
                    name_interval: f.name_interval,
                }
            })
            .collect();
        // RDirNode::adjustPath: abspath minus the parent's, minus the trailing '/'.
        let name = match dd.parent {
            Some(p) => {
                let from = self.dir(p).abspath.len();
                dd.abspath[from..dd.abspath.len() - 1].to_string()
            }
            None => String::new(),
        };
        DirView {
            parent,
            pos: dd.pos,
            spos: dd.spos,
            colour: dd.col,
            radius: dd.dir_radius,
            visible,
            files,
            name,
            depth: dd.depth,
            since_last_node_change: dd.since_last_node_change,
        }
    }
}
