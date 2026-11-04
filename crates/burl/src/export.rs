//! Headless frame export (`-o FILE|-`, phase 5, ticket #19): each tick is simulated, drawn into
//! the offscreen framebuffer and written as one binary PPM (`P6`), the stream Gource pipes to
//! ffmpeg (`PPMWriter`, reference/gource-cpp-dump/src/core/ppm.cpp). `--debug-dump-max-frames N`
//! stops after N frames. Names, date, title, captions, the logo and the background image are
//! drawn; the slider is not yet.

use super::{log_settings, sim_settings};
use burl_conf::settings::{DebugOptions, DisplaySettings, GourceSettings};
use burl_formats::fetch_log;
use burl_gl::{GlContext, HeadlessEgl, Renderer};
use burl_render::{
    background_batch, build_frame, draw_names, logo_batch, world_matrix, FontManager, NameFonts,
    NameSettings, OverlaySettings, Overlays, SceneSettings, SceneTextures, TextureId, FREE_SANS,
    PAGE_SIZE,
};
use burl_sim::Sim;
use burl_util::prng::SimRng;
use std::io::Write;
use std::sync::Arc;

const BEAM: &[u8] = include_bytes!("../../../data/beam.png");
const FILE: &[u8] = include_bytes!("../../../data/file.png");
const USER: &[u8] = include_bytes!("../../../data/user.png");

/// First texture id of the glyph pages (1-3 are the scene graphics).
const FIRST_GLYPH_TEXTURE: u32 = 10;

/// Decode a PNG to RGBA8, top row first.
fn decode(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(png_bytes));
    dec.set_transformations(png::Transformations::EXPAND);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let px = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        c => return Err(format!("unsupported png colour type {c:?}")),
    };
    Ok((info.width, info.height, rgba))
}

/// Binary PPM: the RGBA frame's colour channels, alpha dropped.
fn write_ppm(out: &mut dyn Write, w: u32, h: u32, rgba: &[u8]) -> std::io::Result<()> {
    write!(out, "P6\n{w} {h}\n255\n")?;
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    out.write_all(&rgb)
}

/// Upload the beam, file and user graphics (ids 1-3) and return their handles.
pub fn upload_scene_textures<C: GlContext>(r: &mut Renderer<C>) -> Result<SceneTextures, String> {
    let mut load = |id, bytes| -> Result<f32, String> {
        let (tw, th, px) = decode(bytes)?;
        r.upload_texture(TextureId(id), tw, th, &px, true)?;
        Ok(th as f32 / tw as f32)
    };
    load(1, BEAM)?;
    let file_ratio = load(2, FILE)?;
    let user_ratio = load(3, USER)?;
    Ok(SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio,
        user: TextureId(3),
        user_ratio,
    })
}

pub fn run(
    display: &DisplaySettings,
    repos: &mut [GourceSettings],
    rng: SimRng,
    debug: &DebugOptions,
) -> Result<(), String> {
    let (w, h) = (display.width as u32, display.height as u32);
    let mut r = Renderer::new(HeadlessEgl::new(w, h)?)?;
    let tex = upload_scene_textures(&mut r)?;

    // --font-file is not honoured yet: the bundled FreeSans (the default) is always used.
    let mut fonts = FontManager::new(FREE_SANS, FIRST_GLYPH_TEXTURE)?;

    let name = &display.output_ppm_filename;
    let mut out: Box<dyn Write> = if name == "-" {
        Box::new(std::io::BufWriter::new(std::io::stdout().lock()))
    } else {
        Box::new(std::io::BufWriter::new(
            std::fs::File::create(name).map_err(|_| format!("could not write to '{name}'"))?,
        ))
    };
    let io = |e: std::io::Error| e.to_string();

    let max_frames = debug.dump_max_frames.unwrap_or(0);
    let repo_count = repos.len();
    let mut carry = (rng, -1i64);
    for (i, g) in repos.iter_mut().enumerate() {
        g.apply_shell_fixups(repo_count, true);
        // Gource::init: the default font scale follows the display height (no HiDPI ratio here).
        if g.default_font_scale {
            g.font_scale = (1 + (display.height / 1600).max(0)) as f32;
            g.set_scaled_font_sizes();
        }
        let settings = sim_settings(display, g, i as i32 + 1, repo_count as i32);
        let log = fetch_log(&g.path.clone(), Arc::new(log_settings(g))).map_err(|e| e.0)?;
        let mut sim = Sim::new(settings, log, carry.0).map_err(|e| e.0)?;
        sim.carry_first_rand_frame(carry.1);

        let mut ss = SceneSettings::new(w, h);
        ss.bloom_multiplier = g.bloom_multiplier;
        ss.bloom_intensity = g.bloom_intensity;
        ss.hide_bloom = g.hide_bloom;
        ss.hide_tree = g.hide_tree;
        ss.hide_files = g.hide_files;
        ss.hide_users = g.hide_users;
        ss.user_scale = g.user_scale;
        // --logo / --background-image: PNG only (Gource also reads JPEG and GIF).
        let load_image = |path: &str, id: u32, r: &mut Renderer<_>, mip: bool| {
            let bytes = std::fs::read(path).map_err(|e| format!("could not read '{path}': {e}"))?;
            let (iw, ih, px) = decode(&bytes).map_err(|e| format!("'{path}': {e}"))?;
            r.upload_texture(TextureId(id), iw, ih, &px, mip)?;
            Ok::<_, String>((
                TextureId(id),
                burl_render::glam::Vec2::new(iw as f32, ih as f32),
            ))
        };
        let screen = burl_render::glam::Vec2::new(w as f32, h as f32);
        let background = match g.background_image.as_str() {
            "" => None,
            p => Some(load_image(p, 4, &mut r, true)?),
        };
        let logo = match g.logo.as_str() {
            "" => None,
            p => Some(load_image(p, 5, &mut r, false)?),
        };
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
        let mut name_fonts = NameFonts::new(
            &mut fonts,
            g.scaled_dirname_font_size as u32,
            g.scaled_filename_font_size as u32,
            g.scaled_user_font_size as u32,
        );
        let mut overlays = Overlays::new(
            &mut fonts,
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
        let bg = g.background_colour;
        let clear = [bg[0], bg[1], bg[2], 1.0];

        while !sim.is_finished() {
            sim.update().map_err(|e| e.0)?;
            let scene = sim.scene_view();
            let mut frame = build_frame(&scene, &tex, &ss);
            frame.underlay = background
                .map(|(t, size)| background_batch(t, size, screen))
                .into_iter()
                .collect();
            frame.overlay = logo
                .map(|(t, size)| logo_batch(t, size, screen, g.logo_offset.into()))
                .into_iter()
                .collect();
            fonts.start_buffer();
            draw_names(&mut fonts, &mut name_fonts, &scene, &tex, &ss, &ns);
            frame.names = std::mem::take(&mut fonts.buffer);
            let date = burl_util::timeparse::format_local(scene.time, &os.date_format);
            overlays.draw(&mut fonts, &mut frame.overlay, &scene, &date, &os);
            // Glyphs cached while drawing the names must be on the GPU before the render.
            for u in fonts.take_updates() {
                r.upload_texture(u.id, PAGE_SIZE as u32, PAGE_SIZE as u32, &u.rgba, false)?;
            }
            let world = world_matrix(&scene, &ss);
            r.render(clear, &frame.batches(), world)?;
            write_ppm(&mut out, w, h, &r.read_frame()?).map_err(io)?;
            let last = sim.is_last_frame(max_frames);
            sim.end_frame();
            if last {
                break;
            }
        }
        carry = sim.into_carry();
    }
    out.flush().map_err(io)
}
