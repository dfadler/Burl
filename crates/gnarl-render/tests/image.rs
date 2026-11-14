//! `--logo` / `--background-image` decoding: Gource reads whatever SDL_image reads
//! (texture.cpp `TextureResource::load`); Gnarl reads PNG, JPEG and GIF.

use gnarl_render::decode_image;

fn px(img: &gnarl_render::Image, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.rgba[i..i + 4].try_into().unwrap()
}

fn near(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 12)
}

#[test]
fn jpeg_decodes_to_rgba_top_row_first() {
    let img = decode_image(include_bytes!("images/halves.jpg")).unwrap();
    assert_eq!(
        (img.width, img.height, img.rgba.len()),
        (16, 16, 16 * 16 * 4)
    );
    assert!(
        near(px(&img, 2, 3), [255, 0, 0, 255]),
        "{:?}",
        px(&img, 2, 3)
    );
    assert!(
        near(px(&img, 13, 12), [0, 0, 255, 255]),
        "{:?}",
        px(&img, 13, 12)
    );
}

#[test]
fn grayscale_jpeg_expands_to_rgba() {
    let img = decode_image(include_bytes!("images/halves-gray.jpg")).unwrap();
    let p = px(&img, 2, 3);
    assert!(p[0] == p[1] && p[1] == p[2] && p[3] == 255, "{p:?}");
}

#[test]
fn gif_decodes_first_frame_exactly() {
    let img = decode_image(include_bytes!("images/halves.gif")).unwrap();
    assert_eq!((img.width, img.height), (16, 16));
    assert_eq!(px(&img, 2, 3), [255, 0, 0, 255]);
    assert_eq!(px(&img, 13, 12), [0, 0, 255, 255]);
}

#[test]
fn png_still_decodes() {
    let img = decode_image(gnarl_render::USER_PNG).unwrap();
    assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
}

#[test]
fn unknown_format_is_an_error() {
    assert!(decode_image(b"BM not a supported image").is_err());
}

/// The embedded copies under `assets/` (shipped in the crates.io package) are the `data/` files.
#[test]
fn assets_match_data() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    for (name, embedded) in [
        ("beam.png", gnarl_render::BEAM_PNG),
        ("file.png", gnarl_render::FILE_PNG),
        ("user.png", gnarl_render::USER_PNG),
        ("fonts/FreeSans.ttf", gnarl_render::FREE_SANS),
    ] {
        assert_eq!(std::fs::read(data.join(name)).unwrap(), embedded, "{name}");
    }
}
