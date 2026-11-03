//! Headless smoke render: a synthetic batch through the real GL path, checked in Rust, written as
//! a PNG. `smoke [--size WxH] [--frames N] [--out FILE.png]`. Exits 1 if a pixel check fails.
//! The pattern is asymmetric (red block top-left on blue), so a missing row flip fails the check.
//! With `--frames N` it also prints ms/frame (#19 item 5: llvmpipe's rate, for information).

use burl_gl::{HeadlessEgl, Renderer};
use burl_render::{Font, FontManager, Frame, FREE_SANS};
use glam::{Vec2, Vec4};
use std::time::Instant;

fn main() -> Result<(), String> {
    let (mut size, mut frames, mut out) = ((320u32, 200u32), 1u32, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size WxH")?;
                size = (
                    w.parse().map_err(|e| format!("{e}"))?,
                    h.parse().map_err(|e| format!("{e}"))?,
                );
            }
            "--frames" => {
                frames = value()?.parse().map_err(|e| format!("{e}"))?;
                if frames == 0 {
                    return Err("--frames must be at least 1".into());
                }
            }
            "--out" => out = Some(value()?),
            _ => return Err(format!("unknown argument {a}")),
        }
    }
    let (w, h) = size;
    // The pixel checks need the red block (w/4 x h/4) to cover pixel (1,1) and the corners distinct.
    if w < 8 || h < 8 {
        return Err("--size must be at least 8x8".into());
    }
    let mut r = Renderer::new(HeadlessEgl::new(w, h)?)?;
    println!("GL: {}", r.gl_info()?);

    let (wf, hf) = (w as f32, h as f32);
    // A 1x1 magenta texture on the white file quad exercises upload, binding and sampling.
    let magenta = burl_render::TextureId(1);
    r.upload_texture(magenta, 1, 1, &[255, 0, 255, 255], false)?;
    let mut f = Frame {
        file_graphic: Some(magenta),
        ..Frame::default()
    };
    let tex = burl_render::FULL_TEXCOORD;
    // Top-left red block (edges pass, screen space), a textured "file" with its shadow, and a bloom.
    f.edges.add(
        None,
        Vec2::ZERO,
        Vec2::new(wf / 4.0, hf / 4.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        tex,
    );
    f.files.add(
        None,
        Vec2::new(wf / 2.0, hf / 2.0),
        Vec2::new(wf / 8.0, hf / 8.0),
        Vec4::ONE,
        tex,
    );
    f.bloom.add(
        Vec2::new(wf * 0.6, hf * 0.3),
        Vec2::splat(hf / 4.0),
        Vec4::new(1.0, 1.0, 0.0, 1.0),
        Vec4::new(hf / 8.0, wf * 0.6 + hf / 8.0, hf * 0.3 + hf / 8.0, 0.0),
    );
    // Text: a buffered line (text shader, shadow in the shader) and an immediate line (modulate
    // shader, separate shadow batch), both white on the blue clear colour, in the bottom half.
    let mut fm = FontManager::new(FREE_SANS, 10)?;
    let mut font: Font = fm.grab(h / 10);
    font.shadow = true;
    fm.start_buffer();
    fm.draw_buffered(&font, 4.0, hf * 0.55, "Gource");
    f.names = std::mem::take(&mut fm.buffer);
    fm.draw(&font, &mut f.overlay, 4.0, hf * 0.8, "Gource");
    for u in fm.take_updates() {
        r.upload_texture(u.id, 512, 512, &u.rgba, false)?;
    }
    let batches = f.batches();
    // Files are in world space: pixel space here, as if the camera were the 2D ortho.
    let world = burl_render::ortho_2d(wf, hf);
    let blue = [0.0, 0.0, 1.0, 1.0];

    let start = Instant::now();
    for _ in 0..frames {
        r.render(blue, &batches, world)?;
    }
    let px = r.read_frame()?; // also waits for the GPU
    if frames > 1 {
        println!(
            "{w}x{h}: {:.2} ms/frame over {frames} frames",
            start.elapsed().as_secs_f64() * 1e3 / frames as f64
        );
    }

    let at = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    let checks = [
        ("top-left is the red block", at(1, 1), [255, 0, 0, 255]),
        (
            "file quad samples its texture",
            at(w / 2 + w / 16, h / 2 + h / 16),
            [255, 0, 255, 255],
        ),
        (
            "bottom-right is the clear colour",
            at(w - 1, h - 1),
            [0, 0, 255, 255],
        ),
        (
            "bottom-left is the clear colour",
            at(1, h - 2),
            [0, 0, 255, 255],
        ),
        (
            "top-right is the clear colour",
            at(w - 2, 1),
            [0, 0, 255, 255],
        ),
    ];
    // Rows holding each text line: the pixels left of the first glyph's right edge.
    let text_rows = |y0: f32| {
        let (y0, y1) = (y0 as u32, (y0 + hf / 8.0) as u32);
        (y0..y1.min(h))
            .flat_map(|y| (0..w / 2).map(move |x| (x, y)))
            .map(|(x, y)| at(x, y))
            .collect::<Vec<_>>()
    };
    let (buffered, immediate) = (text_rows(hf * 0.55), text_rows(hf * 0.8));
    let bright = |p: &[[u8; 4]]| p.iter().any(|c| c[0] > 200 && c[1] > 200);
    let dark = |p: &[[u8; 4]]| p.iter().any(|c| c[2] < 200);
    let mut failed = false;
    for (what, ok) in [
        ("buffered text has white glyph pixels", bright(&buffered)),
        ("buffered text has a shadow", dark(&buffered)),
        ("immediate text has white glyph pixels", bright(&immediate)),
        ("immediate text has a shadow", dark(&immediate)),
    ] {
        failed |= !ok;
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
    }
    for (what, got, want) in checks {
        let ok = got == want;
        failed |= !ok;
        println!("{} {what}: {got:?}", if ok { "ok  " } else { "FAIL" });
    }
    if let Some(path) = out {
        let file = std::io::BufWriter::new(
            std::fs::File::create(&path).map_err(|e| format!("{path}: {e}"))?,
        );
        let mut enc = png::Encoder::new(file, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .and_then(|mut wr| wr.write_image_data(&px))
            .map_err(|e| format!("{path}: {e}"))?;
        println!("wrote {path}");
    }
    if failed {
        std::process::exit(1);
    }
    Ok(())
}
