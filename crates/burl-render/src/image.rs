//! Image decoding for `--logo`, `--background-image` and avatars. Gource loads these through
//! SDL_image `IMG_Load` (reference/gource-cpp-dump/src/core/texture.cpp, `TextureResource::load`);
//! here PNG, JPEG and GIF are sniffed from the magic bytes and decoded to RGBA8, top row first.

/// A decoded image: RGBA8, top row first.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn gray_alpha(px: &[u8], channels: usize) -> impl Iterator<Item = [u8; 4]> + '_ {
    px.chunks_exact(channels).map(move |p| match channels {
        1 => [p[0], p[0], p[0], 255],
        2 => [p[0], p[0], p[0], p[1]],
        3 => [p[0], p[1], p[2], 255],
        _ => [p[0], p[1], p[2], p[3]],
    })
}

fn png(bytes: &[u8]) -> Result<Image, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let px = &buf[..info.buffer_size()];
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        c => return Err(format!("unsupported png colour type {c:?}")),
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba: gray_alpha(px, channels).flatten().collect(),
    })
}

fn jpeg(bytes: &[u8]) -> Result<Image, String> {
    let mut dec = jpeg_decoder::Decoder::new(bytes);
    let px = dec.decode().map_err(|e| e.to_string())?;
    let info = dec.info().ok_or("jpeg without header")?;
    let rgba: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => gray_alpha(&px, 1).flatten().collect(),
        jpeg_decoder::PixelFormat::RGB24 => gray_alpha(&px, 3).flatten().collect(),
        f => return Err(format!("unsupported jpeg pixel format {f:?}")),
    };
    Ok(Image {
        width: info.width.into(),
        height: info.height.into(),
        rgba,
    })
}

/// First frame only, as SDL_image's `IMG_Load` returns for a GIF.
fn gif(bytes: &[u8]) -> Result<Image, String> {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut dec = opts.read_info(bytes).map_err(|e| e.to_string())?;
    let frame = dec
        .read_next_frame()
        .map_err(|e| e.to_string())?
        .ok_or("gif without a frame")?;
    Ok(Image {
        width: frame.width.into(),
        height: frame.height.into(),
        rgba: frame.buffer.to_vec(),
    })
}

/// Decode PNG, JPEG or GIF bytes (chosen by magic number, not file name).
pub fn decode_image(bytes: &[u8]) -> Result<Image, String> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => png(bytes),
        [0xFF, 0xD8, 0xFF, ..] => jpeg(bytes),
        [b'G', b'I', b'F', b'8', ..] => gif(bytes),
        _ => Err("unrecognised image format (expected PNG, JPEG or GIF)".into()),
    }
}
