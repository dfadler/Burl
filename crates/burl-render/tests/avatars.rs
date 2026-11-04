//! Avatars (`RUser::assignUserImage`, user.cpp:183-212; directory scan gource_settings.cpp:811-862).

use burl_render::{scan_dir, scan_user_image_dir, Avatars};
use std::path::PathBuf;

const JPG: &[u8] = include_bytes!("images/halves.jpg"); // 16x16
const GIF: &[u8] = include_bytes!("images/halves.gif");

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("burl-avatars-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A 2x4 PNG (ratio 2.0).
fn tall_png() -> Vec<u8> {
    let mut out = vec![];
    let mut enc = png::Encoder::new(&mut out, 2, 4);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&[255; 2 * 4 * 4])
        .unwrap();
    out
}

#[test]
fn scan_keeps_jpg_jpeg_png_keyed_by_stem_case_insensitively() {
    let d = dir("scan");
    for f in [
        "alice.png",
        "Bob.JPG",
        "carol.jpeg",
        "dave.gif",
        "noext",
        "eve.tar.png",
    ] {
        std::fs::write(d.join(f), b"x").unwrap();
    }
    let dd = format!("{}/", d.display());
    let m = scan_user_image_dir(&dd).unwrap();
    let mut keys: Vec<_> = m.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["Bob", "alice", "carol", "eve.tar"]);
    assert_eq!(m["alice"], PathBuf::from(format!("{dd}alice.png")));
}

#[test]
fn own_image_then_default_then_stock() {
    let d = dir("assign");
    std::fs::write(d.join("alice.png"), tall_png()).unwrap();
    std::fs::write(d.join("fallback.gif"), GIF).unwrap();
    let dd = format!("{}/", d.display());
    let def = d.join("fallback.gif");
    let mut a = Avatars::new(&dd, def.to_str().unwrap(), false)
        .unwrap()
        .unwrap();

    let alice = a.assign("alice").unwrap();
    assert_eq!((alice.ratio, alice.white), (2.0, true));
    let bob = a.assign("bob").unwrap(); // no file: the default image
    assert_eq!(bob.ratio, 1.0);
    let carol = a.assign("carol").unwrap();
    assert_eq!(carol.texture, bob.texture); // cache: one texture per file
    assert_ne!(alice.texture, bob.texture);
    assert_eq!(a.take_uploads().len(), 2);
    assert!(a.take_uploads().is_empty());
    assert!(a.take_error().is_none());

    // No default image: an unknown user keeps the stock graphic.
    let mut b = Avatars::new(&dd, "", false).unwrap().unwrap();
    assert!(b.assign("nobody").is_none());
}

#[test]
fn colour_images_keeps_the_hash_colour() {
    let d = dir("colour");
    std::fs::write(d.join("alice.jpg"), JPG).unwrap();
    let dd = format!("{}/", d.display());
    let mut a = Avatars::new(&dd, "", true).unwrap().unwrap();
    assert!(!a.assign("alice").unwrap().white);
}

#[test]
fn no_options_means_no_avatars_and_a_bad_file_is_reported() {
    assert!(Avatars::new("", "", false).unwrap().is_none());
    let d = dir("bad");
    std::fs::write(d.join("alice.png"), b"not an image").unwrap();
    let mut a = Avatars::new(&format!("{}/", d.display()), "", false)
        .unwrap()
        .unwrap();
    assert!(a.assign("alice").is_none());
    assert!(a.take_error().unwrap().contains("alice.png"));
}

/// macOS lists "é" decomposed (e + U+0301); the key is composed so a log's "é" finds the file.
/// The path keeps the listed bytes. Off macOS the key is left as listed (`#ifdef __APPLE__`).
#[test]
fn nfc_key_for_a_decomposed_file_name() {
    let d = dir("nfc");
    let decomposed = "Jose\u{301}";
    std::fs::write(d.join(format!("{decomposed}.png")), b"x").unwrap();
    let dd = format!("{}/", d.display());

    let m = scan_dir(&dd, true).unwrap();
    assert_eq!(m.keys().collect::<Vec<_>>(), ["Jos\u{e9}"]);
    assert_eq!(
        m["Jos\u{e9}"],
        PathBuf::from(format!("{dd}{decomposed}.png"))
    );

    let raw = scan_dir(&dd, false).unwrap();
    assert_eq!(raw.keys().collect::<Vec<_>>(), [decomposed]);
}
