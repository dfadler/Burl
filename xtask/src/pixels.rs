//! Pixel comparison for the display harness (ticket #155, decided in #149). Pure functions only;
//! `display_harness` feeds them real captures.
//!
//! Primary check, golden-free: the same paused frame at the same physical size, captured on a 2x
//! and on a 1x display (see [`Pair`] for why the physical size, not the point size, is held equal).
//! Downscale-and-compare is not used: it passes a window that was rendered at 1x and upscaled.
//!
//! Secondary: [`check_baseline`] compares a capture to a stored "regression baseline" taken from
//! Burl itself on the hosted runner. It says "Burl still looks like it did", never "matches Gource".

use crate::ssim::{ssim, Image};

pub fn crop(img: &Image, x: usize, y: usize, w: usize, h: usize) -> Result<Image, String> {
    if x + w > img.w || y + h > img.h {
        return Err(format!(
            "crop {w}x{h}+{x}+{y} outside the {}x{} image",
            img.w, img.h
        ));
    }
    let mut rgb = Vec::with_capacity(w * h * 3);
    for row in y..y + h {
        rgb.extend_from_slice(&img.rgb[(row * img.w + x) * 3..][..w * 3]);
    }
    Ok(Image { w, h, rgb })
}

/// The window's content area from a window capture: drops the title bar (the top rows) and any
/// side border, keeping the bottom-centre `logical * scale` pixels.
pub fn content(img: &Image, logical: (u32, u32), scale: f64) -> Result<Image, String> {
    let w = (f64::from(logical.0) * scale).round() as usize;
    let h = (f64::from(logical.1) * scale).round() as usize;
    if w > img.w || h > img.h {
        return Err(format!(
            "capture {}x{} is smaller than its content {w}x{h}",
            img.w, img.h
        ));
    }
    crop(img, (img.w - w) / 2, img.h - h, w, h)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Upscale {
    Nearest,
    Bilinear,
}

/// Integer-factor upscale (pixel centres aligned, edges clamped).
pub fn upscale(img: &Image, k: usize, how: Upscale) -> Image {
    let (w, h) = (img.w * k, img.h * k);
    let px = |x: usize, y: usize, c: usize| f64::from(img.rgb[(y * img.w + x) * 3 + c]);
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let v = match how {
                    Upscale::Nearest => px(x / k, y / k, c),
                    Upscale::Bilinear => {
                        let sx = ((x as f64 + 0.5) / k as f64 - 0.5).clamp(0.0, (img.w - 1) as f64);
                        let sy = ((y as f64 + 0.5) / k as f64 - 0.5).clamp(0.0, (img.h - 1) as f64);
                        let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
                        let (x1, y1) = ((x0 + 1).min(img.w - 1), (y0 + 1).min(img.h - 1));
                        let (fx, fy) = (sx - x0 as f64, sy - y0 as f64);
                        px(x0, y0, c) * (1.0 - fx) * (1.0 - fy)
                            + px(x1, y0, c) * fx * (1.0 - fy)
                            + px(x0, y1, c) * (1.0 - fx) * fy
                            + px(x1, y1, c) * fx * fy
                    }
                };
                rgb.push(v.round() as u8);
            }
        }
    }
    Image { w, h, rgb }
}

/// Box (mean) downscale by an integer factor; a size that is not a multiple loses its remainder.
pub fn downscale(img: &Image, k: usize) -> Image {
    let (w, h) = (img.w / k, img.h / k);
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let mut s = 0u32;
                for dy in 0..k {
                    for dx in 0..k {
                        s += u32::from(img.rgb[((y * k + dy) * img.w + x * k + dx) * 3 + c]);
                    }
                }
                rgb.push((f64::from(s) / (k * k) as f64).round() as u8);
            }
        }
    }
    Image { w, h, rgb }
}

/// Why this compares equal *physical* sizes: Burl lays the scene out in physical pixels (a fixed
/// pixel font size and camera zoom), so a 320x180 pt window on a 2x display (640x360 px) and a
/// 640x360 pt window on a 1x display show the same 640x360 px frame, while a 320x180 px frame
/// (what a 1x window of 320x180 pt shows) is framed differently. A native render therefore
/// matches the 1x capture pixel for pixel; a 1x render that macOS upscales cannot.
///
/// Mean absolute RGB difference of two same-size images, in 0..255 levels.
pub fn mae(a: &Image, b: &Image) -> Result<f64, String> {
    if (a.w, a.h) != (b.w, b.h) {
        return Err(format!("size mismatch: {}x{} vs {}x{}", a.w, a.h, b.w, b.h));
    }
    let s: u64 = a
        .rgb
        .iter()
        .zip(&b.rgb)
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    Ok(s as f64 / a.rgb.len() as f64)
}

/// What the pair measures. `native_*` compare the 2x-display capture to the 1x-display capture of
/// the same paused frame at the same physical size. `blurred_*` are the same for a negative
/// control built from the 1x capture: downscale by 2, upscale back (nearest and bilinear; the
/// closer one is kept), i.e. what "rendered at 1x and upscaled" looks like for this exact layout.
///
/// MAE decides, not SSIM: on a dark frame SSIM barely moves for a blur (measured 0.9994 native vs
/// 0.987 blurred), while MAE separates them by more than 20x (0.04 vs 0.8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pair {
    pub native_mae: f64,
    pub native_ssim: f64,
    pub blurred_mae: f64,
    pub blurred_ssim: f64,
}

/// `c2`: content of the capture on the 2x display; `c1`: on the 1x display. Same physical size.
pub fn measure_pair(c1: &Image, c2: &Image) -> Result<Pair, String> {
    if (c1.w, c1.h) != (c2.w, c2.h) {
        return Err(format!(
            "capture sizes differ: 1x display {}x{} px, 2x display {}x{} px",
            c1.w, c1.h, c2.w, c2.h
        ));
    }
    // An odd size loses a row/column in the downscale: compare the common top-left part.
    let small = downscale(c1, 2);
    let (w, h) = (small.w * 2, small.h * 2);
    let c1 = crop(c1, 0, 0, w, h)?;
    let mut blurred = (f64::MAX, 0.0);
    for how in [Upscale::Nearest, Upscale::Bilinear] {
        let up = upscale(&small, 2, how);
        let m = mae(&up, &c1)?;
        if m < blurred.0 {
            blurred = (m, ssim(&up, &c1)?);
        }
    }
    Ok(Pair {
        native_mae: mae(c2, &c1)?,
        native_ssim: ssim(c2, &c1)?,
        blurred_mae: blurred.0,
        blurred_ssim: blurred.1,
    })
}

/// High-frequency content: the MAE between an image and a copy of it that was box-downscaled by 2
/// and upscaled back (the closer of nearest and bilinear). Zero for an image that has no detail
/// finer than 2 px, which is what a 1x render stretched to 2x looks like.
pub fn sharpness(img: &Image) -> Result<f64, String> {
    let small = downscale(img, 2);
    let full = crop(img, 0, 0, small.w * 2, small.h * 2)?;
    let mut best = f64::MAX;
    for how in [Upscale::Nearest, Upscale::Bilinear] {
        best = best.min(mae(&upscale(&small, 2, how), &full)?);
    }
    Ok(best)
}

/// The synthetic blurry control: `img` box-downscaled by 2 and stretched back (bilinear), i.e.
/// what the frame looks like if the detail finer than 2 px was never rendered.
pub fn blurred(img: &Image) -> Image {
    upscale(&downscale(img, 2), 2, Upscale::Bilinear)
}

/// How much finer-than-2px detail `img` has relative to its own blurred copy: `sharpness(img) /
/// sharpness(blurred(img))`. Golden-free and independent of the scene's overall detail level, so
/// it works where the 2x and 1x frames cannot be compared pixel for pixel (a `--high-dpi` window
/// is framed in physical pixels, so the 2x capture shows different content than the 1x one).
/// Measured on real captures (macOS 26, Apple Silicon): genuine 2x 2.27, genuine 1x 2.30; the
/// blurred copy of the 2x 1.43; the 1x capture bilinear-stretched to 2x 1.28; nearest-stretched 0.
pub fn crispness(img: &Image) -> Result<f64, String> {
    let s = sharpness(img)?;
    Ok(s / sharpness(&blurred(img))?.max(f64::MIN_POSITIVE))
}

/// Lowest [`crispness`] of a genuine render; the blurred control must score below it.
pub const MIN_CRISPNESS: f64 = 1.8;

/// Lowest absolute [`sharpness`] a capture needs before its crispness ratio means anything. Real
/// captures measured 1.33 to 3.88 (macOS 26, Apple Silicon, four content captures); this is under
/// a third of the lowest. Without it a near-flat frame whose sparse noise vanishes in the 8-bit
/// downscale has blurred sharpness 0, a huge ratio, and passes.
pub const MIN_SHARPNESS: f64 = 0.4;

/// `Ok((crispness, crispness of its blurred copy))` if `img` is crisp; `Err` if it is blurry. A flat
/// or near-flat frame fails rather than passing vacuously.
pub fn check_crisp(img: &Image) -> Result<(f64, f64), String> {
    let s = sharpness(img)?;
    if s < MIN_SHARPNESS {
        return Err(format!(
            "capture has too little detail to judge: sharpness {s:.3} < {MIN_SHARPNESS} (blank or near-flat frame?)"
        ));
    }
    let (c, control) = (crispness(img)?, crispness(&blurred(img))?);
    if c < MIN_CRISPNESS {
        return Err(format!(
            "capture is not crisp: crispness {c:.2} < {MIN_CRISPNESS} (blurred copy scores {control:.2}); \
             rendered at 1x and upscaled?"
        ));
    }
    Ok((c, control))
}

/// A native pair is the same frame to within capture noise (measured 0.04 on a Mac).
pub const MAX_NATIVE_MAE: f64 = 0.25;
/// The blurred control must be at least this many times worse than the limit above.
pub const MIN_CONTROL_FACTOR: f64 = 2.0;

/// `Err` if the 2x capture is not a native render, or if this frame cannot tell native from
/// blurred at all (a blank or flat frame would pass any comparison).
pub fn check_native_2x(p: &Pair) -> Result<(), String> {
    if p.native_mae > MAX_NATIVE_MAE {
        return Err(format!(
            "2x capture differs from the 1x capture of the same frame (MAE {:.3} > {MAX_NATIVE_MAE}, SSIM {:.4}): \
             blurry, doubled or mis-sized on Retina?",
            p.native_mae, p.native_ssim
        ));
    }
    if p.blurred_mae < MAX_NATIVE_MAE * MIN_CONTROL_FACTOR {
        return Err(format!(
            "check is vacuous on this frame: a blurred copy has MAE {:.3}, under {}x the limit {MAX_NATIVE_MAE}",
            p.blurred_mae, MIN_CONTROL_FACTOR
        ));
    }
    Ok(())
}

/// Lowest SSIM against a regression baseline that still passes (see tools/display-harness/baselines/README.md).
pub const MIN_BASELINE_SSIM: f64 = 0.85;

/// Regression baseline, not parity: SSIM of `got` against a stored capture of Burl itself.
pub fn check_baseline(got: &Image, baseline: &Image, min_ssim: f64) -> Result<f64, String> {
    let s = ssim(got, baseline)?;
    if s < min_ssim {
        return Err(format!(
            "SSIM {s:.4} against the regression baseline is below {min_ssim}"
        ));
    }
    Ok(s)
}

fn luma(px: &[u8]) -> f64 {
    0.299 * f64::from(px[0]) + 0.587 * f64::from(px[1]) + 0.114 * f64::from(px[2])
}

/// Allowed distance of a probe's luma from the median luma of the whole content, in 0..255 levels.
/// The scene is a dark, mostly empty field (~28) with soft glows; measured baseline probes sit
/// within a few levels of the median. 24 is generous on purpose: this catches an unfilled bar or a
/// clear-colour block (black ~0 or white ~255 against a ~28 field), not subtle shading.
pub const FILL_LUMA_TOLERANCE: f64 = 24.0;
/// Inset of the probe points from the content edges, in 1x pixels (scaled by the capture scale):
/// keeps rounded window corners out of the sample.
const FILL_INSET: usize = 12;
/// Caption band: the top 15% of the content.
const CAPTION_BAND: f64 = 0.15;
/// Near-white pixels (all channels above this) in the band are caption text; the dimmer coloured
/// beam is excluded.
const CAPTION_WHITE: u8 = 200;
/// Fewest near-white pixels in the band, at 1x; multiplied by scale squared. The date caption
/// measures several hundred; this is a low floor meaning "some bright text is there".
pub const CAPTION_MIN_PX_1X: f64 = 40.0;
/// The caption's centroid x must be within this fraction of the content width from the centre.
pub const CAPTION_CENTRE_TOL: f64 = 0.05;

/// Coarse structural check on a window's content area (title bar already cropped, see [`content`]):
/// the scene fills the whole window and the date caption is at top-centre. Catches a GL surface or
/// viewport that did not follow a display move (scene in a corner, offset, unfilled bars); says
/// nothing about subtle layout. `Ok` carries the measured numbers for the log, `Err` the same
/// numbers plus the reason, so a failing run can be calibrated.
pub fn check_scene_fills_window(img: &Image, scale: f64) -> Result<String, String> {
    let (w, h) = (img.w, img.h);
    let inset = (FILL_INSET as f64 * scale).round() as usize;
    if w < 2 * inset + 8 || h < 2 * inset + 8 {
        return Err(format!("content {w}x{h} too small to judge"));
    }
    let mut hist = [0usize; 256];
    for px in img.rgb.as_chunks::<3>().0 {
        hist[luma(px).round() as usize] += 1;
    }
    let (mut acc, mut median) = (0, 0);
    for (v, n) in hist.iter().enumerate() {
        acc += n;
        if acc * 2 >= w * h {
            median = v;
            break;
        }
    }
    let median = median as f64;
    // Mean luma of a (2r+1)^2 patch, r = 2 px at 1x, so one noisy pixel does not decide.
    let r = (2.0 * scale).round() as usize;
    let patch = |cx: usize, cy: usize| {
        let (mut s, mut n) = (0.0, 0.0);
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                s += luma(&img.rgb[(y * w + x) * 3..][..3]);
                n += 1.0;
            }
        }
        s / n
    };
    let (l, rt, t, b) = (inset + r, w - 1 - inset - r, inset + r, h - 1 - inset - r);
    let (mx, my) = (w / 2, h / 2);
    // No top-centre probe: the date caption lives there, and at 2x the inset row lands inside the
    // text (hosted macos-15 and macos-26 read luma 99.7 there, every other probe 26.0). The
    // caption check below covers that band, and the top corners still probe the top edge.
    let probes = [
        ("top-left", l, t),
        ("top-right", rt, t),
        ("left", l, my),
        ("right", rt, my),
        ("bottom-left", l, b),
        ("bottom", mx, b),
        ("bottom-right", rt, b),
    ];
    let lumas: Vec<(&str, f64)> = probes.iter().map(|&(n, x, y)| (n, patch(x, y))).collect();
    let band_h = (h as f64 * CAPTION_BAND) as usize;
    let (mut count, mut sum_x) = (0usize, 0usize);
    for y in 0..band_h {
        for x in 0..w {
            if img.rgb[(y * w + x) * 3..][..3]
                .iter()
                .all(|&c| c > CAPTION_WHITE)
            {
                count += 1;
                sum_x += x;
            }
        }
    }
    let min_px = (CAPTION_MIN_PX_1X * scale * scale).ceil() as usize;
    let centroid = if count > 0 {
        sum_x as f64 / count as f64
    } else {
        f64::NAN
    };
    let nums = format!(
        "median luma {median}, probes [{}], caption {count} px (min {min_px}), centroid x {centroid:.1} of width {w} (centre {:.1} +-{:.1})",
        lumas
            .iter()
            .map(|(n, v)| format!("{n} {v:.1}"))
            .collect::<Vec<_>>()
            .join(", "),
        w as f64 / 2.0,
        w as f64 * CAPTION_CENTRE_TOL
    );
    if let Some((n, v)) = lumas
        .iter()
        .find(|(_, v)| (v - median).abs() > FILL_LUMA_TOLERANCE)
    {
        return Err(format!(
            "scene does not fill the window: {n} probe luma {v:.1} is more than {FILL_LUMA_TOLERANCE} from the median: {nums}"
        ));
    }
    if count < min_px {
        return Err(format!("date caption missing at top-centre: {nums}"));
    }
    if (centroid - w as f64 / 2.0).abs() > w as f64 * CAPTION_CENTRE_TOL {
        return Err(format!("date caption is not centred: {nums}"));
    }
    Ok(nums)
}

// ---- is the cursor hidden? (`--grab`, from full-screen captures) ----
//
// Measured on hosted macos-15 and macos-26 (cursor lab, Burl #205): a full-screen `screencapture -C`
// shows the cursor sprite before the press and not while burl holds its grab. `CGCursorIsVisible`
// reads "visible" throughout (winit hides the cursor with an invisible cursor rect), so only the
// pixels can say.

/// Side of the square crop around the cursor's hot spot, in capture pixels.
pub const CURSOR_CROP: usize = 48;
/// How far up and left of the hot spot the crop starts: the arrow hangs down and to the right.
const CURSOR_MARGIN: usize = 8;

/// Luma of a `CURSOR_CROP` square whose top-left is `CURSOR_MARGIN` px up-left of `hot` (the cursor
/// position in capture pixels). `None` when the square leaves the image.
pub fn cursor_crop(img: &Image, hot: (f64, f64)) -> Option<Vec<f64>> {
    let x0 = hot.0.round() as i64 - CURSOR_MARGIN as i64;
    let y0 = hot.1.round() as i64 - CURSOR_MARGIN as i64;
    let side = CURSOR_CROP as i64;
    if x0 < 0 || y0 < 0 || x0 + side > img.w as i64 || y0 + side > img.h as i64 {
        return None;
    }
    let mut v = Vec::with_capacity(CURSOR_CROP * CURSOR_CROP);
    for y in y0..y0 + side {
        for x in x0..x0 + side {
            v.push(luma(&img.rgb[(y as usize * img.w + x as usize) * 3..][..3]));
        }
    }
    Some(v)
}

/// Mean absolute difference of two equal-length crops, in 0..255 levels.
pub fn crop_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f64>() / a.len().max(1) as f64
}

/// The five full-screen captures one press needs, all with the cursor parked at the same point.
pub struct CursorShots<'a> {
    /// With the cursor drawn (`screencapture -C`), before the press.
    pub before: &'a Image,
    /// The same again: its difference from `before` is the capture noise.
    pub before_again: &'a Image,
    /// Without the cursor (no `-C`), before the press: `before` minus this is the sprite.
    pub before_no_cursor: &'a Image,
    /// With the cursor drawn, while the grab is held.
    pub held: &'a Image,
    /// Without the cursor, while the grab is held.
    pub held_no_cursor: &'a Image,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorScores {
    /// Cursor drawn versus not, before the press: how much the sprite shows.
    pub sprite: f64,
    /// Two identical captures: how much the capture itself varies.
    pub noise: f64,
    /// Before the press versus held, both with the cursor drawn: how much the crop changed.
    pub hide: f64,
    /// Cursor drawn versus not, while held: how much of a sprite is still there.
    pub held_sprite: f64,
}

pub fn cursor_scores(s: &CursorShots, hot: (f64, f64)) -> Result<CursorScores, String> {
    let c = |img: &Image| {
        cursor_crop(img, hot).ok_or(format!(
            "the {CURSOR_CROP} px crop at {hot:?} leaves the {}x{} capture",
            img.w, img.h
        ))
    };
    let (a, b, n, h, hn) = (
        c(s.before)?,
        c(s.before_again)?,
        c(s.before_no_cursor)?,
        c(s.held)?,
        c(s.held_no_cursor)?,
    );
    Ok(CursorScores {
        sprite: crop_diff(&a, &n),
        noise: crop_diff(&a, &b),
        hide: crop_diff(&a, &h),
        held_sprite: crop_diff(&h, &hn),
    })
}

/// The cursor must show before the press and be gone while the grab is held. The held capture is
/// judged by its OWN cursor-versus-no-cursor comparison, not by the change from `before` alone: that
/// also moves when the background under the crop changes. A result that cannot be told from "the
/// capture does not show the cursor at all" is an error, never a pass: a vacuous pass would look
/// exactly like a working check.
pub fn check_cursor_hidden(s: &CursorScores) -> Result<String, String> {
    let significant = |v: f64| v > 3.0 * s.noise.max(0.5);
    let nums = format!(
        "sprite before the press {:.2}, capture noise {:.2}, crop change before to held {:.2}, sprite still present while held {:.2}",
        s.sprite, s.noise, s.hide, s.held_sprite
    );
    if !significant(s.sprite) {
        Err(format!(
            "the cursor sprite is not visible in the capture before the press, so a hidden cursor cannot be judged: {nums}"
        ))
    } else if significant(s.held_sprite) {
        Err(format!(
            "the cursor sprite is still present in the held capture (or the background moved under it): {nums}"
        ))
    } else if s.hide <= 0.5 * s.sprite {
        Err(format!(
            "inconclusive: no sprite while held, yet the crop did not change from before the press: {nums}"
        ))
    } else {
        Ok(nums)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text-like test card: thin 1px lines and a diagonal, so a 1x render and a true 2x render of
    /// it genuinely differ the way glyphs do.
    fn scene(k: usize) -> Image {
        let (w, h) = (64 * k, 48 * k);
        let mut rgb = vec![20; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                // positions in 1x-point units
                let (fx, fy) = (x as f64 / k as f64, y as f64 / k as f64);
                let line = fx.fract() < 0.5 && (fx as usize).is_multiple_of(4);
                let diag = ((fx - fy).abs() < 0.6) && fx > 8.0;
                if line || diag {
                    rgb[(y * w + x) * 3..][..3].fill(235);
                }
            }
        }
        Image { w, h, rgb }
    }

    #[test]
    fn crop_and_content_pick_the_bottom_centre() {
        let mut i = Image {
            w: 4,
            h: 3,
            rgb: vec![0; 36],
        };
        i.rgb[(2 * 4 + 1) * 3] = 9; // row 2, col 1
        let c = crop(&i, 1, 2, 2, 1).unwrap();
        assert_eq!((c.w, c.h, c.rgb[0]), (2, 1, 9));
        assert!(crop(&i, 3, 0, 2, 1).is_err());
        let c = content(&i, (2, 1), 1.0).unwrap();
        assert_eq!((c.w, c.h, c.rgb[0]), (2, 1, 9));
        assert!(content(&i, (5, 1), 1.0).is_err());
    }

    #[test]
    fn up_and_downscale_round_trip_on_flat_blocks() {
        let s = scene(1);
        assert_eq!(downscale(&upscale(&s, 2, Upscale::Nearest), 2).rgb, s.rgb);
        let b = upscale(&s, 2, Upscale::Bilinear);
        assert_eq!((b.w, b.h), (128, 96));
    }

    #[test]
    fn identical_native_pair_passes() {
        let p = measure_pair(&scene(2), &scene(2)).unwrap();
        assert!(p.native_ssim > 0.999, "{p:?}");
        assert_eq!(check_native_2x(&p), Ok(()), "{p:?}");
    }

    #[test]
    fn a_blurred_2x_fails_against_the_native_1x_frame() {
        let c1 = scene(2);
        for how in [Upscale::Nearest, Upscale::Bilinear] {
            let fake = upscale(&downscale(&c1, 2), 2, how);
            let p = measure_pair(&c1, &fake).unwrap();
            assert!(p.native_mae > MAX_NATIVE_MAE, "{how:?} {p:?}");
            assert!(check_native_2x(&p).is_err(), "{how:?} {p:?}");
        }
    }

    #[test]
    fn a_shifted_frame_fails() {
        let c1 = scene(2);
        let mut shifted = c1.rgb.clone();
        shifted.rotate_left(10 * 3);
        let p = measure_pair(
            &c1,
            &Image {
                w: c1.w,
                h: c1.h,
                rgb: shifted,
            },
        )
        .unwrap();
        assert!(check_native_2x(&p).is_err(), "{p:?}");
    }

    #[test]
    fn wrong_size_is_an_error() {
        assert!(measure_pair(&scene(1), &scene(2)).is_err());
    }

    #[test]
    fn a_blank_frame_is_vacuous_not_a_pass() {
        let blank = Image {
            w: 64,
            h: 48,
            rgb: vec![7; 64 * 48 * 3],
        };
        let p = measure_pair(&blank, &blank).unwrap();
        assert!(check_native_2x(&p).is_err(), "{p:?}");
    }

    #[test]
    fn baseline_passes_itself_and_fails_a_different_image() {
        let a = scene(2);
        assert_eq!(
            check_baseline(&a, &a, MIN_BASELINE_SSIM).map(|s| s > 0.999),
            Ok(true)
        );
        let b = Image {
            w: a.w,
            h: a.h,
            rgb: vec![20; a.rgb.len()],
        };
        assert!(check_baseline(&b, &a, MIN_BASELINE_SSIM).is_err());
    }

    #[test]
    fn a_native_render_is_crisp_and_its_blurred_copy_is_not() {
        let (c, control) = check_crisp(&scene(2)).unwrap();
        assert!(
            c >= MIN_CRISPNESS && control < MIN_CRISPNESS,
            "{c} {control}"
        );
    }

    #[test]
    fn a_stretched_1x_render_is_not_crisp() {
        for how in [Upscale::Nearest, Upscale::Bilinear] {
            let fake = upscale(&scene(1), 2, how);
            assert!(check_crisp(&fake).is_err(), "{how:?}");
        }
        assert!(check_crisp(&blurred(&scene(2))).is_err());
    }

    #[test]
    fn a_flat_frame_is_vacuous_not_crisp() {
        let flat = Image {
            w: 64,
            h: 48,
            rgb: vec![7; 64 * 48 * 3],
        };
        assert!(check_crisp(&flat).is_err());
    }

    /// Real Burl captures (the regression baselines): the genuine one passes, the same capture
    /// downscaled and stretched back, and stretched from half size, fail.
    #[test]
    fn real_capture_passes_and_its_blurred_copies_fail() {
        let real = crate::ssim::decode(include_bytes!(
            "../../tools/display-harness/baselines/macos-26/1x.png"
        ))
        .unwrap();
        let (c, control) = check_crisp(&real).unwrap();
        assert!(
            c > MIN_CRISPNESS && control < MIN_CRISPNESS,
            "{c} {control}"
        );
        assert!(check_crisp(&blurred(&real)).is_err());
        assert!(check_crisp(&upscale(&downscale(&real, 2), 2, Upscale::Nearest)).is_err());
    }

    /// 640x360 (at k=1) dark field with a white caption block at x offset `cx` (centre), plus a
    /// dim coloured beam near the top that must not count as caption.
    fn fill_scene(k: usize, cx: usize, caption: bool) -> Image {
        let (w, h) = (640 * k, 360 * k);
        let mut img = Image {
            w,
            h,
            rgb: [28u8, 28, 28].repeat(w * h),
        };
        let mut rect = |x0: usize, y0: usize, x1: usize, y1: usize, c: [u8; 3]| {
            for y in y0..y1 {
                for x in x0..x1 {
                    img.rgb[(y * w + x) * 3..][..3].copy_from_slice(&c);
                }
            }
        };
        rect(460 * k, 0, 480 * k, 20 * k, [200, 120, 120]);
        if caption {
            rect(
                (cx - 100) * k,
                18 * k,
                (cx + 100) * k,
                30 * k,
                [255, 255, 255],
            );
        }
        img
    }

    #[test]
    fn a_filled_scene_with_centred_caption_passes_at_1x_and_2x() {
        for k in [1, 2] {
            let r = check_scene_fills_window(&fill_scene(k, 320, true), k as f64);
            assert!(r.is_ok(), "{k}x: {r:?}");
        }
    }

    /// The first hosted run failed on a probe at top-centre that fell inside the caption text at 2x
    /// (luma 99.7 against a median of 26). A caption covering the inset row must not fail the fill.
    #[test]
    fn a_caption_covering_the_top_inset_row_is_not_an_unfilled_edge() {
        let mut img = fill_scene(2, 320, true);
        for y in 20..36 {
            for x in 540..740 {
                img.rgb[(y * img.w + x) * 3..][..3].fill(255);
            }
        }
        let r = check_scene_fills_window(&img, 2.0);
        assert!(r.is_ok(), "{r:?}");
    }

    #[test]
    fn a_white_bar_on_the_top_edge_fails() {
        let mut img = fill_scene(1, 320, true);
        for y in 0..40 {
            for x in 0..img.w {
                img.rgb[(y * img.w + x) * 3..][..3].fill(255);
            }
        }
        let e = check_scene_fills_window(&img, 1.0).unwrap_err();
        assert!(e.contains("does not fill"), "{e}");
    }

    #[test]
    fn a_scene_in_the_top_left_quarter_fails() {
        // Scene drawn at half size in the top-left; the rest is the clear colour (black).
        let s = fill_scene(1, 160, true);
        let mut img = Image {
            w: s.w,
            h: s.h,
            rgb: vec![0; s.rgb.len()],
        };
        for y in 0..s.h / 2 {
            for x in 0..s.w / 2 {
                img.rgb[(y * s.w + x) * 3..][..3]
                    .copy_from_slice(&s.rgb[(y * 2 * s.w + x * 2) * 3..][..3]);
            }
        }
        // Black bars dominate: the median itself is black, so the lit quarter is what stands out.
        assert!(check_scene_fills_window(&img, 1.0).is_err());
    }

    #[test]
    fn a_white_bar_on_one_edge_fails() {
        let mut img = fill_scene(1, 320, true);
        for y in 0..img.h {
            for x in img.w - 40..img.w {
                img.rgb[(y * img.w + x) * 3..][..3].fill(255);
            }
        }
        let e = check_scene_fills_window(&img, 1.0).unwrap_err();
        assert!(e.contains("does not fill"), "{e}");
    }

    #[test]
    fn a_missing_caption_fails() {
        let e = check_scene_fills_window(&fill_scene(1, 320, false), 1.0).unwrap_err();
        assert!(e.contains("missing"), "{e}");
    }

    #[test]
    fn an_off_centre_caption_fails() {
        let e = check_scene_fills_window(&fill_scene(1, 200, true), 1.0).unwrap_err();
        assert!(e.contains("not centred"), "{e}");
    }

    #[test]
    fn real_captures_fill_the_window_with_a_centred_caption() {
        for png in [
            &include_bytes!("../../tools/display-harness/baselines/macos-26/1x.png")[..],
            &include_bytes!("../../tools/display-harness/baselines/macos-26/2x.png")[..],
        ] {
            let img = crate::ssim::decode(png).unwrap();
            let r = check_scene_fills_window(&img, 1.0);
            eprintln!("baseline: {r:?}");
            assert!(r.is_ok(), "{r:?}");
        }
    }

    #[test]
    fn a_near_flat_frame_with_sparse_noise_is_not_crisp() {
        // Isolated level-1 pixels on black, one per 2x2 block: each block averages 0.25 and rounds
        // to 0 on the way down, so the blurred copy is perfectly flat (sharpness 0) while the
        // frame itself has a tiny nonzero sharpness. The ratio would be huge and pass vacuously.
        let (w, h) = (64, 64);
        let mut rgb = vec![0u8; w * h * 3];
        for y in (0..h).step_by(8) {
            for x in (0..w).step_by(8) {
                rgb[(y * w + x) * 3] = 1;
            }
        }
        let img = Image { w, h, rgb };
        assert!(sharpness(&img).unwrap() > 0.0);
        assert_eq!(sharpness(&blurred(&img)).unwrap(), 0.0);
        assert!(check_crisp(&img).is_err());
    }

    /// A 200x200 capture of flat `bg` luma, with a bright 20x20 "cursor sprite" whose hot spot is at
    /// (100, 100) when `sprite` is set.
    fn screen(bg: u8, sprite: bool) -> Image {
        let (w, h) = (200, 200);
        let mut rgb = vec![bg; w * h * 3];
        if sprite {
            for y in 100..120 {
                for x in 100..120 {
                    rgb[(y * w + x) * 3..][..3].fill(255);
                }
            }
        }
        Image { w, h, rgb }
    }

    const HOT: (f64, f64) = (100.0, 100.0);

    fn scores(
        before: &Image,
        again: &Image,
        none: &Image,
        held: &Image,
        held_none: &Image,
    ) -> CursorScores {
        cursor_scores(
            &CursorShots {
                before,
                before_again: again,
                before_no_cursor: none,
                held,
                held_no_cursor: held_none,
            },
            HOT,
        )
        .unwrap()
    }

    #[test]
    fn a_cursor_that_shows_before_and_is_gone_while_held_passes() {
        let (with, without) = (screen(100, true), screen(100, false));
        let s = scores(&with, &with, &without, &without, &without);
        assert_eq!(s.noise, 0.0);
        assert!(
            s.sprite > 10.0 && s.hide > 10.0 && s.held_sprite == 0.0,
            "{s:?}"
        );
        assert!(check_cursor_hidden(&s).is_ok());
    }

    #[test]
    fn a_cursor_still_drawn_while_held_fails() {
        let (with, without) = (screen(100, true), screen(100, false));
        let s = scores(&with, &with, &without, &with, &without);
        let e = check_cursor_hidden(&s).unwrap_err();
        assert!(e.contains("still present in the held capture"), "{e}");
    }

    #[test]
    fn a_capture_that_never_shows_the_cursor_is_an_error_not_a_pass() {
        let without = screen(100, false);
        let s = scores(&without, &without, &without, &without, &without);
        let e = check_cursor_hidden(&s).unwrap_err();
        assert!(e.contains("not visible"), "{e}");
    }

    #[test]
    fn a_changed_background_under_a_retained_sprite_is_not_reported_as_hidden() {
        // The crop differs a lot from before the press, but the held cursor-versus-no-cursor
        // comparison still shows the sprite: the cursor did not go away.
        let (with, without) = (screen(100, true), screen(100, false));
        let (held, held_none) = (screen(160, true), screen(160, false));
        let s = scores(&with, &with, &without, &held, &held_none);
        assert!(s.hide > 0.5 * s.sprite && s.held_sprite > 10.0, "{s:?}");
        let e = check_cursor_hidden(&s).unwrap_err();
        assert!(e.contains("still present in the held capture"), "{e}");
    }

    #[test]
    fn capture_noise_as_large_as_the_sprite_cannot_prove_it_was_visible() {
        let (with, without) = (screen(100, true), screen(100, false));
        // two "identical" captures that differ as much as the sprite does
        let s = scores(&with, &without, &without, &without, &without);
        assert!(check_cursor_hidden(&s).is_err());
    }

    #[test]
    fn a_crop_that_leaves_the_capture_is_reported() {
        let with = screen(100, true);
        let near_edge = CursorShots {
            before: &with,
            before_again: &with,
            before_no_cursor: &with,
            held: &with,
            held_no_cursor: &with,
        };
        let e = cursor_scores(&near_edge, (190.0, 100.0)).unwrap_err();
        assert!(e.contains("leaves"), "{e}");
    }
}
