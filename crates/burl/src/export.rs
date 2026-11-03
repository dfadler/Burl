//! Headless frame export (`-o FILE|-`, phase 5, ticket #19): each tick is simulated, drawn into
//! the offscreen framebuffer and written as one binary PPM (`P6`), the stream Gource pipes to
//! ffmpeg (`PPMWriter`, reference/gource-cpp-dump/src/core/ppm.cpp). `--debug-dump-max-frames N`
//! stops after N frames. Overlays, text and the background image are not drawn yet.

use super::{log_settings, sim_settings};
use burl_conf::settings::{DebugOptions, DisplaySettings, GourceSettings};
use burl_formats::fetch_log;
use burl_gl::{HeadlessEgl, Renderer};
use burl_render::{build_frame, world_matrix, SceneSettings, SceneTextures, TextureId};
use burl_sim::Sim;
use burl_util::prng::SimRng;
use std::io::Write;
use std::sync::Arc;

const BEAM: &[u8] = include_bytes!("../../../data/beam.png");
const FILE: &[u8] = include_bytes!("../../../data/file.png");
const USER: &[u8] = include_bytes!("../../../data/user.png");

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

pub fn run(
    display: &DisplaySettings,
    repos: &mut [GourceSettings],
    rng: SimRng,
    debug: &DebugOptions,
) -> Result<(), String> {
    let (w, h) = (display.width as u32, display.height as u32);
    let mut r = Renderer::new(HeadlessEgl::new(w, h)?)?;
    let mut load = |id, bytes| -> Result<f32, String> {
        let (tw, th, px) = decode(bytes)?;
        r.upload_texture(TextureId(id), tw, th, &px, true)?;
        Ok(th as f32 / tw as f32)
    };
    load(1, BEAM)?;
    let file_ratio = load(2, FILE)?;
    let user_ratio = load(3, USER)?;
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio,
        user: TextureId(3),
        user_ratio,
    };

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
        let settings = sim_settings(display, g, i as i32 + 1, repo_count as i32);
        let log = fetch_log(&g.path.clone(), Arc::new(log_settings(g))).map_err(|e| e.0)?;
        let mut sim = Sim::new(settings, log, carry.0).map_err(|e| e.0)?;
        sim.carry_first_rand_frame(carry.1);

        let mut ss = SceneSettings::new(w, h);
        ss.bloom_multiplier = g.bloom_multiplier;
        ss.bloom_intensity = g.bloom_intensity;
        ss.hide_bloom = g.hide_bloom;
        ss.user_scale = g.user_scale;
        let bg = g.background_colour;
        let clear = [bg[0], bg[1], bg[2], 1.0];

        while !sim.is_finished() {
            sim.update().map_err(|e| e.0)?;
            let scene = sim.scene_view();
            let frame = build_frame(&scene, &tex, &ss);
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
