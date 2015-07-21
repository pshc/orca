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

    let mut cursor = 0;
    for ch in "Okay then".chars() {
        try!(face.load_char(ch as usize, ft::face::RENDER));

        let slot = face.glyph();
        try!(slot.render_glyph(ft::RenderMode::Normal));
        let bitmap = slot.bitmap();

        let buf = bitmap.buffer();
        let pitch = bitmap.pitch();
        let w = bitmap.width();
        let right = cmp::min(cursor + w, dest.width() as i32);
        let bottom = cmp::min(bitmap.rows(), dest.height() as i32);
        for y in 0..bottom {
            for x in cursor..right {
                let luma = buf[(y * pitch + x - cursor) as usize];
                dest.put_pixel(x as u32, y as u32, image::Luma([luma]));
            }
        }
        cursor += w + 1;
    }

    Ok(())
}


fn main() {
    let mut img = image::ImageBuffer::new(40, 20);

    blit(&mut img).unwrap();

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();
}
