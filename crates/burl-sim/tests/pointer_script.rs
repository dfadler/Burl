//! Scripted-pointer behaviour that the oracle goldens do not pin.

use burl_formats::{fetch_log, LogSettings};
use burl_sim::{DumpOptions, PointerEvent, PointerKind, Sim, SimSettings};
use burl_util::prng::SimRng;
use std::sync::Arc;

fn sim(tag: &str, settings: SimSettings) -> Sim {
    let path = std::env::temp_dir().join(format!("burl-ptr-{tag}-{}.log", std::process::id()));
    std::fs::write(
        &path,
        "1277787455|bob|A|a/b.txt\n1277787456|amy|A|c/d.txt\n",
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
        ..settings
    };
    Sim::new(settings, log, SimRng::from_script(vec![1, 2, 3]).unwrap()).unwrap()
}

fn selected(sim: &mut Sim) -> usize {
    sim.scene_view().users.iter().filter(|u| u.selected).count()
}

fn tab(frame: i32) -> PointerEvent {
    PointerEvent {
        frame,
        kind: PointerKind::Tab,
    }
}

/// `keyPress` returns first under `--disable-input`, so a scripted Tab selects nobody.
#[test]
fn scripted_tab_is_ignored_under_disable_input() {
    for (disable_input, want) in [(true, 0), (false, 1)] {
        let mut s = sim(
            &format!("tab{disable_input}"),
            SimSettings {
                disable_input,
                ..SimSettings::default()
            },
        );
        for _ in 0..100 {
            s.update().unwrap();
            s.end_frame();
        }
        s.apply_pointer(&tab(0));
        assert_eq!(selected(&mut s), want, "disable_input = {disable_input}");
    }
}

/// `gDump.pointer_pos` outlives the section: a Tab consumed in section 1 is not replayed in
/// section 2, whose frame counter restarts at 0.
#[test]
fn the_script_cursor_carries_across_sections() {
    let o = DumpOptions {
        every: 1,
        max_frames: 130,
        prng: "script".into(),
        prng_seed: None,
        pointer: Some(vec![tab(100)]),
    };
    let mut cursor = 0;
    let mut dumps = Vec::new();
    for _ in 0..2 {
        let mut out = Vec::new();
        sim("carry", SimSettings::default())
            .run_section(&mut out, &o, &mut cursor)
            .unwrap();
        dumps.push(String::from_utf8(out).unwrap());
    }
    assert!(dumps[0].contains("\"selected_user\":\""), "{}", dumps[0]);
    assert!(!dumps[1].contains("\"selected_user\":\""), "{}", dumps[1]);
}
