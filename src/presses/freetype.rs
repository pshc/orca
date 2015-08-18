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
}

impl<'a> super::Press for FreeTypePress<'a> {
    fn blit_str<I: super::Paper>(&self, text: &str, mut pen: (i32, i32), dest: &mut I)
        -> FtResult<()> {

        let (dest_w, dest_h) = dest.dimensions();

        pen.1 += self.line_height;
        for ch in text.chars() {
            try!(self.face.load_char(ch as usize, ft::face::RENDER));

            let slot = self.face.glyph();
            let bitmap = slot.bitmap();
            let buf = bitmap.buffer();

            let (left, top) = (pen.0 + slot.bitmap_left(), pen.1 - slot.bitmap_top());
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
            pen.0 += (slot.advance().x / 64) as i32;
            pen.1 += (slot.advance().y / 64) as i32;
        }

        Ok(())
    }

    fn measure_str(&self, text: &str) -> FtResult<(u32, u32)> {
        let mut pen_x = 0;

        for ch in text.chars() {
            try!(self.face.load_char(ch as usize, ft::face::DEFAULT));
            let slot = self.face.glyph();
            pen_x += (slot.advance().x / 64) as i32;
        }

        let pen = (cmp::max(pen_x, 0) as u32, 10);
        Ok(pen)
    }
}
