pub mod freetype;

use image::{GenericImage, ImageBuffer, Luma};

pub use self::freetype::FreeTypePress;

/// Anything that can be inked in grayscale.
pub trait Paper : GenericImage<Pixel=Luma<u8>> {
}

impl Paper for ImageBuffer<Luma<u8>, Vec<u8>> {
}
