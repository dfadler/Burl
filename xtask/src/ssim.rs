//! The visual-equivalence comparator (ticket #12): structural SSIM between two frames, plus the
//! helpers around it (PPM stream frame cutting, the measured-threshold rule).
//!
//! SSIM is Wang et al. 2004 on Rec. 601 luma (0..255): an 11x11 Gaussian window (sigma 1.5),
//! K1 0.01, K2 0.03, averaged over every window that fits fully inside the image.

use std::path::Path;

pub struct Image {
    pub w: usize,
    pub h: usize,
    /// RGB8, top row first.
    pub rgb: Vec<u8>,
}

/// Headroom under the worst measured oracle self-SSIM (#12 item 2).
pub const MARGIN: f64 = 0.02;

const WIN: usize = 11;
const SIGMA: f64 = 1.5;

/// A PNG (RGB or RGBA, 8 bit) or one binary PPM (`P6`).
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    if bytes.starts_with(b"\x89PNG") {
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::EXPAND);
        let mut r = dec.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; r.output_buffer_size().ok_or("png too large")?];
        let i = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
        if i.bit_depth != png::BitDepth::Eight {
            return Err("only 8-bit png is supported".into());
        }
        let px = &buf[..i.buffer_size()];
        let rgb = match i.color_type {
            png::ColorType::Rgb => px.to_vec(),
            png::ColorType::Rgba => px.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect(),
            c => return Err(format!("unsupported png colour type {c:?}")),
        };
        return Ok(Image {
            w: i.width as usize,
            h: i.height as usize,
            rgb,
        });
    }
    let frames = ppm_frames(bytes)?;
    match frames[..] {
        [f] => Ok(Image {
            w: f.0,
            h: f.1,
            rgb: f.2.to_vec(),
        }),
        _ => Err("expected exactly one PPM frame".into()),
    }
}

/// Every frame of a concatenated `P6` stream (what `-o -` writes) as `(w, h, rgb bytes)`.
/// `(width, height, rgb bytes)`.
pub type Frame<'a> = (usize, usize, &'a [u8]);

pub fn ppm_frames(bytes: &[u8]) -> Result<Vec<Frame<'_>>, String> {
    let mut frames = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        let mut tok = Vec::new();
        let mut p = pos;
        while tok.len() < 4 {
            let start = p;
            while p < bytes.len() && !bytes[p].is_ascii_whitespace() {
                p += 1;
            }
            tok.push(std::str::from_utf8(&bytes[start..p]).map_err(|_| "bad PPM header")?);
            p += 1; // exactly one whitespace byte after each token; after maxval it ends the header
            if p > bytes.len() {
                return Err("truncated PPM header".into());
            }
        }
        let num = |s: &str| {
            s.parse::<usize>()
                .map_err(|_| format!("bad PPM header field {s:?}"))
        };
        if tok[0] != "P6" || num(tok[3])? != 255 {
            return Err("only 8-bit binary PPM (P6) is supported".into());
        }
        let (w, h) = (num(tok[1])?, num(tok[2])?);
        let end = p + w * h * 3;
        if end > bytes.len() {
            return Err(format!("truncated PPM frame {}", frames.len()));
        }
        frames.push((w, h, &bytes[p..end]));
        pos = end;
    }
    Ok(frames)
}

pub fn write_png(path: &Path, w: usize, h: usize, rgb: &[u8]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(rgb))
        .map_err(|e| e.to_string())
}

fn luma(i: &Image) -> Vec<f64> {
    i.rgb
        .chunks(3)
        .map(|p| 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]))
        .collect()
}

fn kernel() -> [f64; WIN] {
    let mut k = [0.0; WIN];
    for (i, v) in k.iter_mut().enumerate() {
        let d = i as f64 - (WIN / 2) as f64;
        *v = (-d * d / (2.0 * SIGMA * SIGMA)).exp();
    }
    let s: f64 = k.iter().sum();
    k.map(|v| v / s)
}

/// Separable Gaussian filter, valid region only: `(w - 10) x (h - 10)` values.
fn blur(src: &[f64], w: usize, h: usize, k: &[f64; WIN]) -> Vec<f64> {
    let (ow, oh) = (w - WIN + 1, h - WIN + 1);
    let mut tmp = vec![0.0; ow * h];
    for y in 0..h {
        for x in 0..ow {
            tmp[y * ow + x] = (0..WIN).map(|i| k[i] * src[y * w + x + i]).sum();
        }
    }
    let mut out = vec![0.0; ow * oh];
    for y in 0..oh {
        for x in 0..ow {
            out[y * ow + x] = (0..WIN).map(|i| k[i] * tmp[(y + i) * ow + x]).sum();
        }
    }
    out
}

/// Mean SSIM of two same-size frames; 1.0 is identical.
pub fn ssim(a: &Image, b: &Image) -> Result<f64, String> {
    if (a.w, a.h) != (b.w, b.h) {
        return Err(format!("size mismatch: {}x{} vs {}x{}", a.w, a.h, b.w, b.h));
    }
    if a.w < WIN || a.h < WIN {
        return Err(format!("frame smaller than the {WIN}x{WIN} window"));
    }
    let (x, y) = (luma(a), luma(b));
    let k = kernel();
    let (w, h) = (a.w, a.h);
    let sq = |v: &[f64], u: &[f64]| v.iter().zip(u).map(|(p, q)| p * q).collect::<Vec<_>>();
    let (mx, my) = (blur(&x, w, h, &k), blur(&y, w, h, &k));
    let (exx, eyy, exy) = (
        blur(&sq(&x, &x), w, h, &k),
        blur(&sq(&y, &y), w, h, &k),
        blur(&sq(&x, &y), w, h, &k),
    );
    let (c1, c2) = ((0.01_f64 * 255.0).powi(2), (0.03_f64 * 255.0).powi(2));
    let sum: f64 = (0..mx.len())
        .map(|i| {
            let (vx, vy, cxy) = (
                exx[i] - mx[i] * mx[i],
                eyy[i] - my[i] * my[i],
                exy[i] - mx[i] * my[i],
            );
            (2.0 * mx[i] * my[i] + c1) * (2.0 * cxy + c2)
                / ((mx[i] * mx[i] + my[i] * my[i] + c1) * (vx + vy + c2))
        })
        .sum();
    Ok(sum / mx.len() as f64)
}

/// Mean Rec. 601 luma (0..255) of a frame: the bloom luminance check (#53, D11).
pub fn mean_luma(i: &Image) -> f64 {
    let l = luma(i);
    l.iter().sum::<f64>() / l.len() as f64
}

/// The #12 rule: worst observed self-SSIM minus [`MARGIN`]. At least 10 runs are the caller's job.
pub fn threshold(self_ssims: &[f64]) -> Option<f64> {
    self_ssims
        .iter()
        .copied()
        .reduce(f64::min)
        .map(|m| m - MARGIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_luma_is_rec601() {
        let i = Image { w: 2, h: 1, rgb: vec![255, 0, 0, 0, 0, 0] };
        assert!((mean_luma(&i) - 0.299 * 255.0 / 2.0).abs() < 1e-9);
    }

    /// A 64x64 frame with a bright square at (ox, oy) on a mid-grey ramp.
    fn scene(ox: usize, oy: usize) -> Image {
        let mut rgb = vec![0; 64 * 64 * 3];
        for y in 0..64 {
            for x in 0..64 {
                let v = if (ox..ox + 16).contains(&x) && (oy..oy + 16).contains(&y) {
                    250
                } else {
                    (x * 2) as u8
                };
                rgb[(y * 64 + x) * 3..][..3].fill(v);
            }
        }
        Image { w: 64, h: 64, rgb }
    }

    #[test]
    fn identical_frames_score_one() {
        assert!((ssim(&scene(10, 10), &scene(10, 10)).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn moved_structure_scores_clearly_lower_than_a_one_pixel_shift() {
        let base = scene(10, 10);
        let far = ssim(&base, &scene(34, 30)).unwrap();
        let near = ssim(&base, &scene(11, 10)).unwrap();
        assert!(far < 0.8, "far {far}");
        assert!(near > far + 0.1, "near {near} far {far}");
    }

    #[test]
    fn known_value_for_constant_frames() {
        // Flat 0 vs flat 255: only the luminance term survives: C1/(255^2+C1), C2 terms cancel.
        let (z, f) = (
            Image {
                w: 16,
                h: 16,
                rgb: vec![0; 768],
            },
            Image {
                w: 16,
                h: 16,
                rgb: vec![255; 768],
            },
        );
        let (c1, c2) = (6.5025_f64, 58.5225_f64);
        let want = (c1 / (255.0 * 255.0 + c1)) * (c2 / c2);
        assert!((ssim(&z, &f).unwrap() - want).abs() < 1e-9);
    }

    #[test]
    fn size_mismatch_and_tiny_frames_are_errors() {
        let a = scene(0, 0);
        let b = Image {
            w: 32,
            h: 32,
            rgb: vec![0; 32 * 32 * 3],
        };
        assert!(ssim(&a, &b).is_err());
        let t = Image {
            w: 4,
            h: 4,
            rgb: vec![0; 48],
        };
        assert!(ssim(&t, &t).is_err());
    }

    #[test]
    fn ppm_stream_splits_into_frames_and_rejects_truncation() {
        let mut s = b"P6\n2 1\n255\n".to_vec();
        s.extend([1, 2, 3, 4, 5, 6]);
        s.extend(b"P6\n2 1\n255\n");
        s.extend([7, 8, 9, 10, 11, 12]);
        let f = ppm_frames(&s).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[1].2, &[7, 8, 9, 10, 11, 12]);
        assert!(ppm_frames(&s[..s.len() - 1]).is_err());
        assert!(decode(&s).is_err(), "two frames are not one image");
        assert_eq!(decode(&s[..17]).unwrap().rgb, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn png_round_trips_through_decode() {
        let i = scene(5, 5);
        let p = std::env::temp_dir().join(format!("burl-ssim-{}.png", std::process::id()));
        write_png(&p, i.w, i.h, &i.rgb).unwrap();
        let back = decode(&std::fs::read(&p).unwrap()).unwrap();
        std::fs::remove_file(&p).ok();
        assert_eq!((back.w, back.h, back.rgb), (i.w, i.h, i.rgb));
    }

    #[test]
    fn threshold_is_worst_minus_margin() {
        assert!((threshold(&[0.99, 0.95, 0.97]).unwrap() - 0.93).abs() < 1e-12);
        assert_eq!(threshold(&[]), None);
    }
}
