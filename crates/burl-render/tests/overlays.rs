//! Date, title and caption overlays (no GL): placement, fade colour, row bookkeeping and flags.
//! `BURL_BLESS=1 cargo test -p burl-render --test overlays` rewrites the snapshot.

use burl_render::*;
use burl_sim::view::*;
use glam::Vec3;

fn scene(time: i64, seekable: bool, captions: &[(&str, f32)], activated: usize) -> SceneView {
    SceneView {
        camera: CameraView {
            pos: Vec3::new(0.0, 0.0, -300.0),
            target: Vec3::ZERO,
            max_distance: 10000.0,
        },
        dirs: vec![],
        users: vec![],
        time,
        seekable,
        captions: captions
            .iter()
            .map(|&(t, a)| CaptionView {
                text: t.into(),
                alpha: a,
            })
            .collect(),
        captions_activated: activated,
        key_events: vec![],
    }
}

fn settings() -> OverlaySettings {
    OverlaySettings {
        width: 640.0,
        height: 480.0,
        hide_date: false,
        hide_progress: false,
        title: String::new(),
        date_format: String::new(),
        font_colour: Vec3::ONE,
        caption_colour: Vec3::new(1.0, 1.0, 0.0),
        caption_offset: 0,
        show_key: false,
        font_scale: 1.0,
        scaled_font_size: 16.0,
        file_extension_fallback: false,
    }
}

struct Rig {
    mgr: FontManager,
    ov: Overlays,
}

fn rig() -> Rig {
    let mut mgr = FontManager::new(FREE_SANS, 100).unwrap();
    let ov = Overlays::new(&mut mgr, 16, 18);
    Rig { mgr, ov }
}

impl Rig {
    fn draw(&mut self, s: &SceneView, date: &str, o: &OverlaySettings) -> Vec<Batch> {
        let mut out = vec![];
        self.ov.draw(&mut self.mgr, &mut out, s, date, o);
        out
    }
}

/// The first vertex of the first batch of text `n` (counting shadow+colour pairs as one).
fn first_pos(b: &Batch) -> [f32; 2] {
    let Vertices::Quads(v) = &b.vertices else {
        panic!()
    };
    v[0].pos
}

fn colours(b: &[Batch]) -> Vec<[f32; 4]> {
    b.iter()
        .map(|b| match &b.vertices {
            Vertices::Quads(v) => v[0].colour,
            _ => panic!(),
        })
        .collect()
}

#[test]
fn overlays_match_snapshot() {
    let mut r = rig();
    let mut o = settings();
    o.title = "Title".into();
    let b = r.draw(
        &scene(1, true, &[("Hello", 0.5), ("Bye", 1.0)], 2),
        "Monday",
        &o,
    );
    let mut got = String::new();
    for b in &b {
        got += &format!(
            "{:?} {:?} tex={:?}\n",
            b.pass,
            b.pipeline,
            b.texture.map(|t| t.0)
        );
        if let Vertices::Quads(v) = &b.vertices {
            for v in v {
                got += &format!("  {:?} {:?} {:?}\n", v.pos, v.colour, v.uv);
            }
        }
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/snapshots/overlays_frame.txt"
    );
    if matches!(std::env::var("BURL_BLESS").as_deref(), Ok("1")) {
        std::fs::write(path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(path).unwrap());
}

#[test]
fn date_is_centred_and_does_not_wobble() {
    let mut r = rig();
    let o = settings();
    let at = |r: &mut Rig, d: &str| first_pos(&r.draw(&scene(1, false, &[], 0), d, &o)[0]);
    let a = at(&mut r, "Monday");
    // A width change of <= 5 px of the half width keeps the old offset (same text again).
    assert_eq!(a, at(&mut r, "Monday"));
    let w = r.mgr.width(r.ov.medium_font(), "Monday");
    // Half the width left of the centre, as an integer.
    let half = ((w as i32) as f64 * 0.5) as i32;
    assert!(
        (a[0] - (320 - half) as f32).abs() <= 3.0,
        "{a:?} half={half}"
    );
    // Time 0 clears the date.
    assert!(r.draw(&scene(0, false, &[], 0), "x", &o).is_empty());
}

#[test]
fn flags_title_and_captions() {
    let mut r = rig();
    let mut o = settings();
    // Date: shadow + colour batch; nothing else.
    assert_eq!(r.draw(&scene(1, false, &[], 0), "d", &o).len(), 2);
    o.hide_date = true;
    assert!(r.draw(&scene(1, false, &[], 0), "d", &o).is_empty());
    o.title = "T".into();
    let b = r.draw(&scene(1, false, &[], 0), "d", &o);
    assert_eq!(b.len(), 2);
    // Bottom-aligned at (10, height - 10): the glyph sits above the baseline.
    assert!(first_pos(&b[0])[0] >= 10.0 && first_pos(&b[0])[1] < 470.0);
    // Caption colour carries the fade alpha (shadow first, then colour).
    o.title.clear();
    let b = r.draw(&scene(1, false, &[("Hi", 0.25)], 1), "d", &o);
    assert_eq!(colours(&b)[1], [1.0, 1.0, 0.0, 0.25]);
}

#[test]
fn captions_stack_upwards_and_keep_their_rows() {
    let mut r = rig();
    let o = settings();
    let y = |b: &[Batch], n: usize| first_pos(&b[n]);
    // Two captions at once: the second is one caption-height above the first.
    let two = r.draw(&scene(0, false, &[("a", 1.0), ("b", 1.0)], 2), "", &o);
    let (y0, y1) = (y(&two, 0)[1], y(&two, 2)[1]);
    assert!(y1 < y0, "{y0} {y1}");
    // The first expires: the second keeps its row (not shifted down)...
    let one = r.draw(&scene(0, false, &[("b", 1.0)], 2), "", &o);
    assert_eq!(y(&one, 0)[1], y1);
    // ...and a new caption takes the freed (lower) row.
    let three = r.draw(&scene(0, false, &[("b", 1.0), ("c", 1.0)], 3), "", &o);
    assert_eq!(y(&three, 2)[1], y0);
}

#[test]
fn seekable_logs_lift_captions_above_the_slider() {
    let mut a = rig();
    let mut b = rig();
    let mut o = settings();
    let cap = |r: &mut Rig, seek, o: &OverlaySettings| {
        first_pos(&r.draw(&scene(0, seek, &[("a", 1.0)], 1), "", o)[0])[1]
    };
    // Not seekable: height - maxHeight - 20 = ~450; seekable: height - 70 - 35 = 375.
    let (no, yes) = (cap(&mut a, false, &o), cap(&mut b, true, &o));
    assert!(yes < no - 50.0, "{yes} {no}");
    // --hide progress turns the slider (and the lift) off again.
    o.hide_progress = true;
    assert_eq!(cap(&mut rig(), true, &o), no);
}

#[test]
fn caption_offset_modes() {
    let x = |off| {
        let mut o = settings();
        o.caption_offset = off;
        first_pos(&rig().draw(&scene(0, false, &[("abc", 1.0)], 1), "", &o)[0])[0]
    };
    assert!(x(100) >= 100.0 && x(100) < 110.0);
    assert!(x(-100) > 440.0 && x(-100) < 540.0);
    assert!((x(0) - 320.0).abs() < 30.0);
}

fn inc(path: &str, c: [f32; 3]) -> KeyEvent {
    KeyEvent::Inc {
        path: path.into(),
        colour: Vec3::from_array(c),
    }
}

/// One frame of the key: a 1/60 s tick first (as the sim orders it), then `events`.
fn key_frame(r: &mut Rig, o: &OverlaySettings, events: Vec<KeyEvent>) -> Vec<Batch> {
    let mut s = scene(0, false, &[], 0);
    s.key_events = std::iter::once(KeyEvent::Tick(1.0 / 60.0))
        .chain(events)
        .collect();
    r.draw(&s, "", o)
}

/// The gradient bars of a frame: (right-hand colour, top y), in draw order.
fn bars(b: &[Batch]) -> Vec<([f32; 4], f32)> {
    b.iter()
        .filter_map(|b| match &b.vertices {
            Vertices::Quads(v) if b.texture.is_none() && v[0].colour[0] != v[2].colour[0] => {
                Some((v[2].colour, v[0].pos[1]))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn key_lists_extensions_by_count_and_fades_in() {
    let mut r = rig();
    let mut o = settings();
    o.show_key = true;
    let (red, blue) = ([1.0, 0.0, 0.0], [0.2, 0.0, 1.0]);
    key_frame(
        &mut r,
        &o,
        vec![inc("a/x.py", blue), inc("a/y.rs", red), inc("b/z.rs", red)],
    );
    // The first tick recounts (setShow zeroed the interval), the second fades in.
    key_frame(&mut r, &o, vec![]);
    let b = key_frame(&mut r, &o, vec![]);
    let now = bars(&b);
    // rs (2 files) sorts above py (1), each at offset_y = size + 6 = 22 steps.
    assert_eq!(now.len(), 2);
    assert_eq!(now[0].0[..3], red);
    assert_eq!(now[1].0[..3], blue);
    assert_eq!((now[0].1, now[1].1), (22.0, 44.0));
    assert!(now[0].0[3] > 0.0 && now[0].0[3] < 0.1, "fading in");
    // After a second it is opaque: per entry a shadow, a bar, the ext label and the count with its text shadow.
    let mut b = vec![];
    for _ in 0..70 {
        b = key_frame(&mut r, &o, vec![]);
    }
    assert_eq!(bars(&b)[0].0[3], 1.0);
    // The shadow is the bar's rectangle at (3, 3), black at a third of the opacity.
    assert_eq!(b[0].offset, [3.0, 3.0]);
    assert_eq!(colours(&b[..1])[0], [0.0, 0.0, 0.0, 0.333]);
    assert_eq!(b.len(), 2 * (1 + 1 + 1 + 2));
}

#[test]
fn key_is_hidden_without_the_flag_and_drops_empty_entries() {
    let mut r = rig();
    let mut o = settings();
    key_frame(&mut r, &o, vec![inc("x.rs", [1.0; 3])]);
    for _ in 0..90 {
        assert!(key_frame(&mut r, &o, vec![]).is_empty());
    }
    o.show_key = true;
    for _ in 0..90 {
        key_frame(&mut r, &o, vec![]);
    }
    assert_eq!(bars(&key_frame(&mut r, &o, vec![])).len(), 1);
    // The last file goes: the entry fades out over a second, then disappears.
    key_frame(
        &mut r,
        &o,
        vec![KeyEvent::Dec {
            path: "x.rs".into(),
        }],
    );
    assert_eq!(bars(&key_frame(&mut r, &o, vec![])).len(), 1);
    for _ in 0..70 {
        key_frame(&mut r, &o, vec![]);
    }
    assert!(key_frame(&mut r, &o, vec![]).is_empty());
}

#[test]
fn key_shows_only_the_entries_that_fit_the_window() {
    let mut r = rig();
    let mut o = settings();
    o.show_key = true;
    o.height = 190.0; // (190 - 150) / 20 = 2 rows
    key_frame(
        &mut r,
        &o,
        vec![
            inc("a.c", [1.0; 3]),
            inc("b.h", [1.0; 3]),
            inc("c.rs", [1.0; 3]),
        ],
    );
    key_frame(&mut r, &o, vec![]);
    assert_eq!(bars(&key_frame(&mut r, &o, vec![])).len(), 2);
}
