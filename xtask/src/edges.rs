//! Edge-profile width: the soft-edge / doubled-line half of the real-panel crispness check (#210).
//! Pure functions; `crate::pixels::crispness` (finer-than-2px detail) says "detail exists", this
//! says "the edges are as steep as one physical pixel allows".
//!
//! Every monotone luma run along a row or column whose step is at least [`MIN_CONTRAST`] is one
//! edge; its width is the 10-90% rise distance in physical pixels, interpolated. A native render
//! has antialiased glyph and node edges about one pixel wide; a 1x render stretched by the
//! compositor spreads each over two or more; a doubled line (the stroke drawn again one pixel
//! over) turns every edge into a two-step ramp, also two or more.

use crate::ssim::Image;

/// Smallest luma step that counts as an edge; the scene's soft glows are well below it.
pub const MIN_CONTRAST: f64 = 80.0;

fn luma_plane(img: &Image) -> Vec<f64> {
    img.rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]))
        .collect()
}

/// Rise widths of every edge in one line of luma samples.
fn line_edges(v: &[f64], out: &mut Vec<f64>) {
    let mut i = 0;
    while i + 1 < v.len() {
        if v[i + 1] == v[i] {
            i += 1;
            continue;
        }
        let s = (v[i + 1] - v[i]).signum();
        let mut j = i + 1;
        while j + 1 < v.len() && v[j + 1] != v[j] && (v[j + 1] - v[j]).signum() == s {
            j += 1;
        }
        let d = v[j] - v[i];
        if d.abs() >= MIN_CONTRAST {
            // position (in samples) where the run first reaches fraction f of its step
            let at = |f: f64| {
                let t = v[i] + f * d;
                let k = (i..j).find(|&k| (v[k + 1] - t) * s >= 0.0).unwrap_or(j - 1);
                k as f64 + (t - v[k]) / (v[k + 1] - v[k])
            };
            out.push(at(0.9) - at(0.1));
        }
        i = j;
    }
}

/// Rise width of every edge, horizontal and vertical.
pub fn edge_widths(img: &Image) -> Vec<f64> {
    let l = luma_plane(img);
    let mut out = Vec::new();
    for y in 0..img.h {
        line_edges(&l[y * img.w..][..img.w], &mut out);
    }
    let mut col = vec![0.0; img.h];
    for x in 0..img.w {
        for (y, c) in col.iter_mut().enumerate() {
            *c = l[y * img.w + x];
        }
        line_edges(&col, &mut out);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeStats {
    pub edges: usize,
    pub median: f64,
    /// Share of edges no wider than [`SHARP_PX`].
    pub sharp: f64,
}

/// An edge this narrow (physical pixels) is as steep as an antialiased native edge gets.
pub const SHARP_PX: f64 = 1.25;
/// Fewest edges a capture needs before the statistics mean anything (blank frame, no caption).
pub const MIN_EDGES: usize = 200;

pub fn edge_stats(img: &Image) -> EdgeStats {
    let mut w = edge_widths(img);
    w.sort_by(f64::total_cmp);
    let n = w.len();
    EdgeStats {
        edges: n,
        median: if n == 0 { f64::NAN } else { w[n / 2] },
        sharp: w.iter().filter(|x| **x <= SHARP_PX).count() as f64 / n.max(1) as f64,
    }
}

/// Widest median edge of a native render. Measured on the real regression baselines (640x360,
/// macos-15 and macos-26, 1x and 2x display): 1.55 to 1.56; the same frames bilinear-stretched 2x:
/// 2.83 to 2.87; blurred (box down, bilinear up): 2.92 to 2.96.
pub const MAX_MEDIAN_PX: f64 = 2.0;
/// Lowest share of sharp edges of a native render. Real baselines: 0.29 to 0.31; the same frames
/// stretched 2x 0.0004 to 0.0015, blurred 0.013 to 0.019, doubled (averaged with a copy shifted
/// one pixel right) 0.135 to 0.164. The median barely moves for a doubled frame (1.60), this does.
pub const MIN_SHARP: f64 = 0.20;

/// `Ok(stats)` if the edges are as steep as a native render's; `Err` names the failed rule and the
/// numbers. Not detected: a stroke drawn uniformly thicker (both its edges stay steep), and a
/// nearest-neighbour stretch (steep but blocky; `pixels::check_crisp` catches that one).
pub fn check_edges(img: &Image) -> Result<EdgeStats, String> {
    let s = edge_stats(img);
    let nums = format!(
        "{} edges, median rise {:.2} px, {:.0}% within {SHARP_PX} px",
        s.edges,
        s.median,
        s.sharp * 100.0
    );
    if s.edges < MIN_EDGES {
        Err(format!(
            "too few edges to judge ({} < {MIN_EDGES}; blank frame, no caption?)",
            s.edges
        ))
    } else if s.median > MAX_MEDIAN_PX {
        Err(format!(
            "soft edges: {nums}; median over {MAX_MEDIAN_PX} px, rendered at 1x and upscaled?"
        ))
    } else if s.sharp < MIN_SHARP {
        Err(format!(
            "doubled or smeared edges: {nums}; under {:.0}% sharp",
            MIN_SHARP * 100.0
        ))
    } else {
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::{blurred, upscale, Upscale};

    /// Card of 1.5 px bars and a diagonal at fractional positions, drawn at 4x supersampling and
    /// box-averaged, i.e. a native render at `k`x with antialiased edges.
    fn card(k: usize) -> Image {
        let (w, h, ss) = (96 * k, 64 * k, 4);
        let mut rgb = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0u32;
                for sy in 0..ss {
                    for sx in 0..ss {
                        let fx = (x * ss + sx) as f64 / (ss * k) as f64;
                        let fy = (y * ss + sy) as f64 / (ss * k) as f64;
                        let bar = (fx % 7.3) < 1.5;
                        let hbar = (fy % 9.1) < 1.5 && fx > 40.0;
                        let diag = (fx - fy * 1.3 - 6.0).abs() < 0.8 && fx > 8.0;
                        acc += if bar || hbar || diag { 235 } else { 20 };
                    }
                }
                rgb.extend([(acc / (ss * ss) as u32) as u8; 3]);
            }
        }
        Image { w, h, rgb }
    }

    /// Every pixel averaged with its left neighbour: each stroke drawn again one pixel over.
    fn doubled(img: &Image) -> Image {
        let mut rgb = img.rgb.clone();
        for y in 0..img.h {
            for x in 1..img.w {
                for c in 0..3 {
                    let at = |x| u16::from(img.rgb[(y * img.w + x) * 3 + c]);
                    rgb[(y * img.w + x) * 3 + c] = ((at(x) + at(x - 1)) / 2) as u8;
                }
            }
        }
        Image {
            w: img.w,
            h: img.h,
            rgb,
        }
    }

    fn real(bytes: &[u8]) -> Image {
        crate::ssim::decode(bytes).unwrap()
    }

    #[test]
    fn rise_width_of_a_step_and_a_ramp() {
        let mut w = Vec::new();
        line_edges(&[0.0, 0.0, 255.0, 255.0], &mut w);
        assert!((w[0] - 0.8).abs() < 1e-9, "{w:?}");
        w.clear();
        line_edges(&[0.0, 64.0, 128.0, 192.0, 255.0], &mut w);
        assert!((w[0] - 3.197).abs() < 0.01, "{w:?}");
        w.clear();
        line_edges(&[0.0, 40.0, 79.0, 0.0], &mut w); // rises 79, under MIN_CONTRAST
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn a_native_render_passes() {
        for k in [1, 2] {
            let r = check_edges(&card(k));
            assert!(r.is_ok(), "{k}x: {r:?}");
        }
    }

    #[test]
    fn a_1x_render_stretched_by_the_compositor_fails() {
        let fake = upscale(&card(1), 2, Upscale::Bilinear);
        let e = check_edges(&fake).unwrap_err();
        assert!(e.contains("soft edges"), "{e}");
        assert!(check_edges(&blurred(&card(2))).is_err());
    }

    #[test]
    fn a_doubled_line_render_fails() {
        for k in [1, 2] {
            assert!(check_edges(&doubled(&card(k))).is_err(), "{k}x");
        }
    }

    #[test]
    fn a_flat_frame_is_not_a_pass() {
        let flat = Image {
            w: 64,
            h: 48,
            rgb: vec![7; 64 * 48 * 3],
        };
        assert!(check_edges(&flat).unwrap_err().contains("too few"));
    }

    /// The calibration frames (real Gnarl, 640x360): genuine passes, the same frames stretched,
    /// blurred or doubled fail.
    #[test]
    fn real_captures_pass_and_their_degraded_copies_fail() {
        for b in [
            &include_bytes!("../../tools/display-harness/baselines/macos-15/1x.png")[..],
            include_bytes!("../../tools/display-harness/baselines/macos-15/2x.png"),
            include_bytes!("../../tools/display-harness/baselines/macos-26/1x.png"),
            include_bytes!("../../tools/display-harness/baselines/macos-26/2x.png"),
        ] {
            let img = real(b);
            assert!(check_edges(&img).is_ok(), "{:?}", check_edges(&img));
            assert!(check_edges(&upscale(&img, 2, Upscale::Bilinear)).is_err());
            assert!(check_edges(&blurred(&img)).is_err());
            assert!(check_edges(&doubled(&img)).is_err());
        }
    }
}
