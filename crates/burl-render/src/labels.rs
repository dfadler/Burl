//! Directory, file and user name labels: the `fontmanager.startBuffer` .. `commitBuffer` block of
//! `Gource::draw` (src/gource.cpp:2495-2527) with `RDirNode::drawNames/drawDirName` (dirnode.cpp),
//! `Pawn::drawName`, `RFile::drawNameText` and `RUser::drawNameText`.
//!
//! The selected user's and file's labels are drawn again last, over everything, as in
//! `Gource::draw` (they go through the same text buffer here, not a second immediate-mode pass).
//!
//! Not ported: the `in_frustum` test of file names (an off-screen label is clipped anyway).

use crate::scene::{camera, label_pos};
use crate::{Font, FontManager, SceneSettings, SceneTextures};
use burl_sim::view::SceneView;
use glam::{Vec2, Vec3};

/// The settings the labels read.
#[derive(Clone, Copy, Debug)]
pub struct NameSettings {
    pub hide_dirnames: bool,
    pub hide_filenames: bool,
    pub hide_usernames: bool,
    pub dir_name_depth: i32,
    pub dir_name_position: f32,
    pub highlight_dirs: bool,
    pub highlight_all_users: bool,
    /// `--file-extensions`: label files with their extension only.
    pub file_extensions: bool,
    /// `--file-extension-fallback`: a file without an extension is its own extension.
    pub file_extension_fallback: bool,
    pub dir_colour: Vec3,
    pub filename_colour: Vec3,
    pub highlight_colour: Vec3,
    /// `--selection-colour`: the selected user's and file's label.
    pub selection_colour: Vec3,
}

impl Default for NameSettings {
    /// `setGourceDefaults`.
    fn default() -> Self {
        Self {
            hide_dirnames: false,
            hide_filenames: false,
            hide_usernames: false,
            dir_name_depth: 0,
            dir_name_position: 0.5,
            highlight_dirs: false,
            highlight_all_users: false,
            file_extensions: false,
            file_extension_fallback: false,
            dir_colour: Vec3::ONE,
            filename_colour: Vec3::ONE,
            highlight_colour: Vec3::ONE,
            selection_colour: Vec3::new(1.0, 1.0, 0.3),
        }
    }
}

/// The three label fonts (`fontdirname`, `file_font`, `RUser::font`), grabbed once.
pub struct NameFonts {
    dir: Font,
    file: Font,
    user: Font,
    /// `RUser::updateFont` and `file_selected_font`: 18 pt whatever `--font-scale` says.
    selected_user: Font,
    selected_file: Font,
}

/// The unscaled size of a selected pawn's label.
const SELECTED_FONT_SIZE: u32 = 18;

impl NameFonts {
    /// Sizes are `scaled_dirname_font_size`, `scaled_filename_font_size`, `scaled_user_font_size`.
    pub fn new(mgr: &mut FontManager, dir: u32, file: u32, user: u32) -> Self {
        let (mut dir, mut file, mut user) = (mgr.grab(dir), mgr.grab(file), mgr.grab(user));
        for f in [&mut dir, &mut file, &mut user] {
            f.shadow = true;
        }
        user.align_top = false;
        let (mut selected_user, mut selected_file) =
            (mgr.grab(SELECTED_FONT_SIZE), mgr.grab(SELECTED_FONT_SIZE));
        for f in [&mut selected_user, &mut selected_file] {
            f.shadow = true;
        }
        selected_user.align_top = false;
        selected_file.round = false;
        Self {
            dir,
            file,
            user,
            selected_user,
            selected_file,
        }
    }
}

/// `Pawn::drawName`: the label's opacity from its fade clock, or `None` when it is not drawn
/// (`nameVisible`). Ports the comparisons literally, including the gap at `done == 1.0`.
pub fn name_alpha(nametime: f32, name_interval: f32) -> Option<f32> {
    if name_interval < 0.0 {
        return None;
    }
    let done = nametime - name_interval;
    Some(if done < 1.0 {
        done
    } else if done > 1.0 && done < nametime - 1.0 {
        1.0
    } else {
        nametime - done
    })
}

/// `RFile::ext`: the text after the last `.`, or the whole name with the fallback.
pub(crate) fn extension(name: &str, fallback: bool) -> &str {
    match name.rfind('.') {
        Some(i) if i + 1 != name.len() => &name[i + 1..],
        _ if fallback => name,
        _ => "",
    }
}

/// Appends every label to `mgr.buffer` in draw order: per directory (pre-order) its name then its
/// files' names, then the users in name order.
pub fn draw_names(
    mgr: &mut FontManager,
    fonts: &mut NameFonts,
    scene: &SceneView,
    _tex: &SceneTextures,
    s: &SceneSettings,
    n: &NameSettings,
) {
    let cam = camera(scene);
    let screen = |p: Vec2| {
        cam.project(Vec3::new(p.x, p.y, 0.0), s.width, s.height)
            .truncate()
    };

    // The labels are only ever drawn through the text shader, so shadows come from the shader.
    fonts.dir.round = false;
    fonts.dir.set_colour(n.dir_colour.extend(1.0));
    for d in &scene.dirs {
        let hidden_depth = n.dir_name_depth > 0 && n.dir_name_depth < d.depth - 1;
        let age = d.since_last_node_change;
        let stale = !n.highlight_dirs && age > 5.0;
        if let (Some(p), true) = (
            d.parent,
            d.visible && !n.hide_dirnames && !hidden_depth && !stale,
        ) {
            let alpha = if n.highlight_dirs {
                1.0
            } else {
                (5.0 - age).max(0.0) / 5.0
            };
            let at = label_pos(
                screen(scene.dirs[p].pos),
                screen(d.pos),
                screen(d.spos),
                n.dir_name_position,
            );
            fonts.dir.set_alpha(alpha);
            mgr.draw_buffered(&fonts.dir, at.x, at.y, &d.name);
        }

        if !(n.hide_filenames || s.hide_files) {
            fonts.file.set_colour(n.filename_colour.extend(1.0));
            for f in d.files.iter().filter(|f| !f.selected) {
                let Some(alpha) = name_alpha(f.nametime, f.name_interval) else {
                    continue;
                };
                if alpha <= 0.01 {
                    continue;
                }
                // RFile::calcScreenPos
                let at = screen(f.pos + Vec2::new(5.5, 0.0) - Vec2::new(0.0, 1.0));
                fonts.file.set_alpha(alpha);
                let text = if n.file_extensions {
                    extension(&f.name, n.file_extension_fallback)
                } else {
                    &f.name
                };
                mgr.draw_buffered(&fonts.file, at.x, at.y, text);
            }
        }
    }

    let show_users = !(n.hide_usernames || s.hide_users);
    for u in scene.users.iter().filter(|u| show_users && !u.selected) {
        // RUser::nameVisible shows highlighted users even after their name has faded; the
        // (negative) fade value is then unused because the user's own alpha is drawn.
        let always = n.highlight_all_users || u.highlighted;
        let fade = name_alpha(u.nametime, u.name_interval);
        let alpha = match fade {
            Some(a) if a > 0.0 || always => a,
            None if always => 0.0,
            _ => continue,
        };
        // RUser::calcScreenPos: above the sprite, centred on the name's width.
        let at = screen(u.pos - Vec2::new(0.0, u.size * u.graphic_ratio * 0.5));
        let x = at.x - mgr.width(&fonts.user, &u.name) * 0.5;
        let y = at.y - mgr.max_height(&fonts.user);
        let c = if u.highlighted {
            n.highlight_colour
        } else {
            Vec3::ONE
        };
        let a = if always { u.alpha } else { alpha };
        fonts.user.set_colour(c.extend(a));
        mgr.draw_buffered(&fonts.user, x, y, &u.name);
    }

    // the selected names again, over the top
    for u in scene.users.iter().filter(|u| u.selected) {
        let at = screen(u.pos - Vec2::new(0.0, u.size * u.graphic_ratio * 0.5));
        let x = at.x - mgr.width(&fonts.selected_user, &u.name) * 0.5;
        let y = at.y - mgr.max_height(&fonts.selected_user);
        fonts
            .selected_user
            .set_colour(n.selection_colour.extend(u.alpha));
        mgr.draw_buffered(&fonts.selected_user, x, y, &u.name);
    }
    for f in scene
        .dirs
        .iter()
        .flat_map(|d| &d.files)
        .filter(|f| f.selected)
    {
        // RFile::calcScreenPos: the selected label sits 2 units below, not 1
        let at = screen(f.pos + Vec2::new(5.5, 0.0) - Vec2::new(0.0, 2.0));
        fonts
            .selected_file
            .set_colour(n.selection_colour.extend(1.0));
        mgr.draw_buffered(&fonts.selected_file, at.x, at.y, &f.name);
    }
}
