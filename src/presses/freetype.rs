extern crate freetype as ft;

use image;
use std::cmp;

use self::ft::FtResult;

pub struct FreeTypePress<'a> {
    _ft: ft::Library,
    face: ft::Face<'a>,

    line_height: i32,
}

impl<'a> FreeTypePress<'a> {
    pub fn new() -> FtResult<FreeTypePress<'a>> {
        let (ttf, pt, dpi) = ("test.ttf", 12, 96);

        let lib = try!(ft::Library::init());
        let face = try!(lib.new_face(ttf, 0));
        try!(face.set_char_size(pt * 64, 0, dpi, 0));

        let line_height = match face.size_metrics() {
            Some(metrics) => (metrics.height / 64) as i32,
            None => {
                let guess = (pt as f32 * 1.5) as i32;
                warn!("{}: no line height metric available; guessing {}", ttf, guess);
                guess
            }
        };

        Ok(FreeTypePress {
            _ft: lib,
            face: face,
            line_height: line_height,
        })
    }

    pub fn blit_str<I>(&self, text: &str, dest: &mut I) -> FtResult<()>
        where I: image::GenericImage<Pixel=image::Luma<u8>> {

        let (dest_w, dest_h) = dest.dimensions();

        let mut pen_x = 0;
        let mut pen_y = self.line_height;
        for ch in text.chars() {
            try!(self.face.load_char(ch as usize, ft::face::RENDER));

            let slot = self.face.glyph();
            try!(slot.render_glyph(ft::RenderMode::Normal));
            let bitmap = slot.bitmap();
            let buf = bitmap.buffer();

            let (left, top) = (pen_x + slot.bitmap_left(), pen_y - slot.bitmap_top());
            let right = cmp::min(left + bitmap.width(), dest_w as i32);
            let bottom = cmp::min(top + bitmap.rows(), dest_h as i32);
            let (w, h) = (right - left, bottom - top);
            if w > 0 && h > 0 {
                let pitch = bitmap.pitch();
                for y in 0..h {
                    for x in 0..w {
                        // can we do this without bounds checking?
                        let luma = buf[(y * pitch + x) as usize];
                        dest.put_pixel((x + left) as u32, (top + y) as u32, image::Luma([luma]));
                    }
                }
            }
            pen_x += (slot.advance().x / 64) as i32;
            pen_y += (slot.advance().y / 64) as i32;
        }

        Ok(())
    }
}
