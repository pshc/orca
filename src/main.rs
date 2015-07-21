extern crate freetype as ft;
extern crate image;

use ft::FtResult;
use std::cmp;
use std::fs::File;
use std::path::Path;


fn blit<I>(dest: &mut I) -> FtResult<()>
    where I: image::GenericImage<Pixel=image::Luma<u8>> {

    let (ttf, pt, dpi) = ("test.ttf", 12, 96);

    let lib = try!(ft::Library::init());
    let face = try!(lib.new_face(ttf, 0));
    try!(face.set_char_size(pt * 64, 0, dpi, 0));

    let (dest_w, dest_h) = dest.dimensions();

    let mut pen_x = 0;
    let mut pen_y = 15;
    for ch in "Okay then".chars() {
        try!(face.load_char(ch as usize, ft::face::RENDER));

        let slot = face.glyph();
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


fn main() {
    let mut img = image::ImageBuffer::new(40, 20);

    blit(&mut img).unwrap();

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();
}
