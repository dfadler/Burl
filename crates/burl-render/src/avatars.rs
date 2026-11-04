//! User avatars: `--user-image-dir`, `--default-user-image` and `--colour-images`
//! (`RUser::assignUserImage`, src/user.cpp:183-212; the directory scan is
//! `GourceSettings::importGourceSettings`, src/gource_settings.cpp:811-862).
//!
//! Decoding and the per-file cache live here; the GL upload stays with the caller, which takes the
//! pending images each frame (`take_uploads`). The texture cache is keyed by file path, as
//! `TextureManager::grabFile` keys by file name, so users sharing an image share a texture.

use crate::{decode_image, Image, TextureId};
use burl_sim::UserGraphic;
use std::collections::HashMap;
use std::path::PathBuf;

/// First texture id handed to an avatar; far above the scene (1-5) and glyph-page ids.
pub const FIRST_AVATAR_TEXTURE: u32 = 1_000_000;

pub struct Avatars {
    /// User name (file stem) to image file, from `--user-image-dir`.
    dir_map: HashMap<String, PathBuf>,
    default_image: Option<PathBuf>,
    colour_images: bool,
    cache: HashMap<PathBuf, (u32, f32)>,
    pending: Vec<(TextureId, Image)>,
    next: u32,
    error: Option<String>,
}

/// `user_image_map`: `.jpg`, `.jpeg` and `.png` files (extension case-insensitive), keyed by the
/// file name without its extension. `dir` is the canonical value from import, ending in `/`.
pub fn scan_user_image_dir(dir: &str) -> Result<HashMap<String, PathBuf>, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("error reading user-image-dir: {e}"))?;
    let mut map = HashMap::new();
    for entry in rd.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        let Some(dot) = file.rfind('.').filter(|&d| d > 0) else {
            continue;
        };
        let (stem, ext) = file.split_at(dot);
        if matches!(ext.to_lowercase().as_str(), ".jpg" | ".jpeg" | ".png") {
            map.insert(stem.to_string(), PathBuf::from(format!("{dir}{file}")));
        }
    }
    Ok(map)
}

impl Avatars {
    /// `None` when neither option is set: every user keeps the stock `user.png`.
    pub fn new(
        user_image_dir: &str,
        default_user_image: &str,
        colour_images: bool,
    ) -> Result<Option<Self>, String> {
        if user_image_dir.is_empty() && default_user_image.is_empty() {
            return Ok(None);
        }
        let dir_map = if user_image_dir.is_empty() {
            HashMap::new()
        } else {
            scan_user_image_dir(user_image_dir)?
        };
        Ok(Some(Self::with_map(
            dir_map,
            (!default_user_image.is_empty()).then(|| default_user_image.into()),
            colour_images,
        )))
    }

    pub fn with_map(
        dir_map: HashMap<String, PathBuf>,
        default_image: Option<PathBuf>,
        colour_images: bool,
    ) -> Self {
        Self {
            dir_map,
            default_image,
            colour_images,
            cache: HashMap::new(),
            pending: Vec::new(),
            next: FIRST_AVATAR_TEXTURE,
            error: None,
        }
    }

    /// `assignUserImage`: their own image, else the default image, else `None` (stock `user.png`).
    /// A file that cannot be loaded is fatal in C++ (`TextureException`); here the first failure
    /// is kept for `take_error` and the user keeps the stock graphic meanwhile.
    pub fn assign(&mut self, name: &str) -> Option<UserGraphic> {
        let path = self
            .dir_map
            .get(name)
            .or(self.default_image.as_ref())?
            .clone();
        let (id, ratio) = match self.cache.get(&path) {
            Some(&hit) => hit,
            None => match self.load(&path) {
                Ok(new) => new,
                Err(e) => {
                    self.error.get_or_insert(e);
                    return None;
                }
            },
        };
        Some(UserGraphic {
            ratio,
            texture: Some(id),
            white: !self.colour_images,
        })
    }

    fn load(&mut self, path: &PathBuf) -> Result<(u32, f32), String> {
        let shown = path.display();
        let bytes = std::fs::read(path).map_err(|e| format!("could not read '{shown}': {e}"))?;
        let img = decode_image(&bytes).map_err(|e| format!("'{shown}': {e}"))?;
        let entry = (self.next, img.height as f32 / img.width as f32);
        self.next += 1;
        self.pending.push((TextureId(entry.0), img));
        self.cache.insert(path.clone(), entry);
        Ok(entry)
    }

    /// Images decoded since the last call, to upload (with mipmaps) before the next render.
    pub fn take_uploads(&mut self) -> Vec<(TextureId, Image)> {
        std::mem::take(&mut self.pending)
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }
}
