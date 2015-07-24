extern crate freetype as ft;

pub mod freetype;

use self::ft::FtResult;
use image::{GenericImage, ImageBuffer, Luma};

pub use self::freetype::FreeTypePress;

/// Anything that can be inked in grayscale.
pub trait Paper : GenericImage<Pixel=Luma<u8>> {
}

impl Paper for ImageBuffer<Luma<u8>, Vec<u8>> {
}

pub trait Press {
    fn blit_str<I: Paper>(&self, text: &str, dest: &mut I) -> FtResult<()>;
}
