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
    if s == 0.0 {
        return Ok(0.0);
    }
    Ok(s / sharpness(&blurred(img))?.max(f64::MIN_POSITIVE))
}

/// Lowest [`crispness`] of a genuine render; the blurred control must score below it.
pub const MIN_CRISPNESS: f64 = 1.8;

/// `Ok((crispness, control crispness))` if `img` is crisp and its blurred copy is not; `Err` if
/// `img` is blurry, or if the frame is too flat to tell the two apart (the check would be vacuous).
pub fn check_crisp(img: &Image) -> Result<(f64, f64), String> {
    let (c, control) = (crispness(img)?, crispness(&blurred(img))?);
    if control >= MIN_CRISPNESS {
        return Err(format!(
            "crispness check is vacuous on this frame: its blurred copy scores {control:.2} (>= {MIN_CRISPNESS})"
        ));
    }
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
}
