//! The frame both paths draw (`Gource::draw`): the scene plus background, names, date, title,
//! key, captions and logo, built from one set of settings. The exporter and the window each own
//! a [`FrameKit`] and call [`FrameKit::compose`]; only where the pixels go differs.

use super::export::FIRST_GLYPH_TEXTURE;
use burl_conf::settings::GourceSettings;
use burl_gl::{GlContext, Renderer};
use burl_render::glam::Vec2;
use burl_render::{
    background_batch, build_frame, decode_image, draw_names, logo_batch, FontManager, Frame,
    NameFonts, NameSettings, OverlaySettings, Overlays, SceneSettings, SceneTextures, TextureId,
    PAGE_SIZE,
};
use burl_sim::view::SceneView;

type Image = Option<(TextureId, Vec2)>;

pub struct FrameKit {
    name_fonts: NameFonts,
    overlays: Overlays,
    ns: NameSettings,
    os: OverlaySettings,
    background: Image,
    logo: Image,
    logo_offset: Vec2,
    screen: Vec2,
}

/// --logo / --background-image: PNG, JPEG or GIF by magic number.
fn load_image<C: GlContext>(
    r: &mut Renderer<C>,
    path: &str,
    id: u32,
    mip: bool,
) -> Result<Image, String> {
    if path.is_empty() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|e| format!("could not read '{path}': {e}"))?;
    let i = decode_image(&bytes).map_err(|e| format!("'{path}': {e}"))?;
    r.upload_texture(TextureId(id), i.width, i.height, &i.rgba, mip)?;
    Ok(Some((
        TextureId(id),
        Vec2::new(i.width as f32, i.height as f32),
    )))
}

impl FrameKit {
    /// Load the images and build the text state; `g` must already have its font sizes fixed.
    pub fn new<C: GlContext>(
        r: &mut Renderer<C>,
        fonts: &mut FontManager,
        g: &GourceSettings,
        w: u32,
        h: u32,
    ) -> Result<Self, String> {
        let background = load_image(r, &g.background_image, 4, true)?;
        let logo = load_image(r, &g.logo, 5, false)?;
        Ok(Self::with_images(fonts, g, w, h, background, logo))
    }

    fn with_images(
        fonts: &mut FontManager,
        g: &GourceSettings,
        w: u32,
        h: u32,
        background: Image,
        logo: Image,
    ) -> Self {
        let ns = NameSettings {
            hide_dirnames: g.hide_dirnames,
            hide_filenames: g.hide_filenames,
            hide_usernames: g.hide_usernames,
            dir_name_depth: g.dir_name_depth,
            dir_name_position: g.dir_name_position,
            highlight_dirs: g.highlight_dirs,
            highlight_all_users: g.highlight_all_users,
            file_extensions: g.file_extensions,
            file_extension_fallback: g.file_extension_fallback,
            dir_colour: g.dir_colour.into(),
            filename_colour: g.filename_colour.into(),
            highlight_colour: g.highlight_colour.into(),
        };
        let name_fonts = NameFonts::new(
            fonts,
            g.scaled_dirname_font_size as u32,
            g.scaled_filename_font_size as u32,
            g.scaled_user_font_size as u32,
        );
        let overlays = Overlays::new(
            fonts,
            g.scaled_font_size as u32,
            g.caption_size.clamp(1, 100) as u32,
        );
        let os = OverlaySettings {
            width: w as f32,
            height: h as f32,
            hide_date: g.hide_date,
            hide_progress: g.hide_progress,
            title: g.title.clone(),
            date_format: g.date_format.clone(),
            font_colour: g.font_colour.into(),
            caption_colour: g.caption_colour.into(),
            caption_offset: g.caption_offset,
            show_key: g.show_key,
            font_scale: g.font_scale,
            scaled_font_size: g.scaled_font_size as f32,
            file_extension_fallback: g.file_extension_fallback,
        };
        Self {
            name_fonts,
            overlays,
            ns,
            os,
            background,
            logo,
            logo_offset: g.logo_offset.into(),
            screen: Vec2::new(w as f32, h as f32),
        }
    }

    /// Everything one frame draws, with no GL: glyphs it cached are left in `fonts` for
    /// [`upload_glyphs`].
    fn batches(
        &mut self,
        fonts: &mut FontManager,
        scene: &SceneView,
        tex: &SceneTextures,
        ss: &SceneSettings,
    ) -> Frame {
        let mut frame = build_frame(scene, tex, ss);
        frame.underlay = self
            .background
            .map(|(t, size)| background_batch(t, size, self.screen))
            .into_iter()
            .collect();
        frame.overlay = self
            .logo
            .map(|(t, size)| logo_batch(t, size, self.screen, self.logo_offset))
            .into_iter()
            .collect();
        fonts.start_buffer();
        draw_names(fonts, &mut self.name_fonts, scene, tex, ss, &self.ns);
        frame.names = std::mem::take(&mut fonts.buffer);
        let date = burl_util::timeparse::format_local(scene.time, &self.os.date_format);
        self.overlays
            .draw(fonts, &mut frame.overlay, scene, &date, &self.os);
        frame
    }

    /// The frame, ready to render: glyphs cached while drawing are already on the GPU.
    pub fn compose<C: GlContext>(
        &mut self,
        fonts: &mut FontManager,
        r: &mut Renderer<C>,
        scene: &SceneView,
        tex: &SceneTextures,
        ss: &SceneSettings,
    ) -> Result<Frame, String> {
        let frame = self.batches(fonts, scene, tex, ss);
        for u in fonts.take_updates() {
            r.upload_texture(u.id, PAGE_SIZE as u32, PAGE_SIZE as u32, &u.rgba, false)?;
        }
        Ok(frame)
    }
}

/// A fresh glyph cache of `font_file` whose pages are texture ids from [`FIRST_GLYPH_TEXTURE`] up.
pub fn new_fonts(font_file: &str) -> Result<FontManager, String> {
    FontManager::new(
        &burl_render::load_font_file(font_file)?,
        FIRST_GLYPH_TEXTURE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fetch_log, log_settings, sim_settings};
    use burl_conf::settings::DisplaySettings;
    use burl_render::Batch;
    use burl_sim::Sim;
    use burl_util::prng::SimRng;
    use std::sync::Arc;

    /// Batches of the third frame of a one-commit log, drawn the way `compose` does for both paths.
    fn batches(g: &mut GourceSettings, bg: Image, logo: Image) -> Vec<Batch> {
        let path = std::env::temp_dir().join(format!("burl-frame-{}.log", std::process::id()));
        std::fs::write(&path, "1500000000|alice|A|/src/main.rs\n").unwrap();
        g.path = path.to_string_lossy().into();
        g.log_format = "custom".into();
        g.apply_shell_fixups(1, true);
        g.set_scaled_font_sizes();
        let d = DisplaySettings::default();
        let log = fetch_log(&g.path.clone(), Arc::new(log_settings(g))).unwrap();
        let mut sim = Sim::new(
            sim_settings(&d, g, 1, 1),
            log,
            SimRng::new(burl_util::prng::PrngMode::Default, Some(1)),
        )
        .unwrap();
        for _ in 0..3 {
            sim.update().unwrap();
            sim.end_frame();
        }
        let _ = std::fs::remove_file(&path);
        let tex = SceneTextures {
            beam: TextureId(1),
            file: TextureId(2),
            file_ratio: 1.0,
            user: TextureId(3),
        };
        let ss = SceneSettings::new(d.width as u32, d.height as u32);
        let mut fonts = new_fonts(burl_render::DEFAULT_FONT_FILE).unwrap();
        let mut kit =
            FrameKit::with_images(&mut fonts, g, d.width as u32, d.height as u32, bg, logo);
        kit.batches(&mut fonts, &sim.scene_view(), &tex, &ss)
            .batches()
    }

    /// The window and the exporter share this builder, so equal settings give equal batches, and
    /// the batches carry what the plain scene lacks: names, the title, the logo and the background.
    #[test]
    fn shared_builder_draws_text_logo_and_background() {
        let img = Some((TextureId(4), Vec2::new(8.0, 8.0)));
        let plain = batches(&mut GourceSettings::default(), None, None);
        let a = batches(&mut GourceSettings::default(), img, img);
        assert_eq!(a, batches(&mut GourceSettings::default(), img, img));
        assert!(a.len() > plain.len());
        let titled = GourceSettings {
            title: "T".into(),
            ..GourceSettings::default()
        };
        assert_ne!(plain, batches(&mut titled.clone(), None, None));
    }
}
