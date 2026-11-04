//! Burl-only pinning tests for register rows (docs/rust-rewrite/DEVIATIONS.md). No oracle golden.

use burl_formats::{fetch_log, LogSettings};
use burl_sim::{DumpOptions, Sim, SimSettings};
use burl_util::prng::SimRng;
use std::sync::Arc;

/// D2: Gource's frame dump depends on how fast the log loads (`loaded_frame0` is whichever frame
/// the async load finished on); Burl has the log before the first frame, so the first simulated
/// frame is the first loaded frame, in every section record and every frame record.
#[test]
fn deviation_d2_log_is_loaded_before_the_first_frame() {
    let path = std::env::temp_dir().join(format!("burl-d2-{}.log", std::process::id()));
    std::fs::write(
        &path,
        "1277787455|bob|A|a/b.txt\n1277787456|bob|A|c/d.txt\n",
    )
    .unwrap();
    let log = fetch_log(
        path.to_str().unwrap(),
        Arc::new(LogSettings {
            log_format: "custom".into(),
            ..LogSettings::default()
        }),
    )
    .unwrap();
    let _ = std::fs::remove_file(&path);
    let settings = SimSettings {
        disable_auto_rotate: true,
        ..SimSettings::default()
    };
    let mut sim = Sim::new(settings, log, SimRng::from_script(vec![1, 2, 3]).unwrap()).unwrap();
    let o = DumpOptions {
        every: 1,
        max_frames: 30,
        prng: "script".into(),
        prng_seed: None,
        pointer: None,
    };
    let mut out = Vec::new();
    sim.run_dump(&mut out, &o).unwrap();
    let text = String::from_utf8(out).unwrap();
    let frames: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("\"type\":\"frame\""))
        .collect();
    assert!(frames.len() >= 2, "{text}");
    assert!(frames[0].contains("\"frame\":0,"), "{}", frames[0]);
    for f in &frames {
        assert!(f.contains("\"loaded_frame0\":0,"), "{f}");
    }
}
