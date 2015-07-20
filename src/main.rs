extern crate freetype as ft;
extern crate image;

use ft::FtResult;
use std::fs::File;
use std::path::Path;


fn blit<I>(img: &mut I) -> FtResult<()>
    where I: image::GenericImage<Pixel=image::Luma<u8>> {

    let (ttf, pt, dpi) = ("test.ttf", 12, 96);

    let lib = try!(ft::Library::init());
    let face = try!(lib.new_face(ttf, 0));
    try!(face.set_char_size(pt * 64, 0, dpi, 0));

    let mut cursor = 0u32;
    for &ch in ['O', 'K'].iter() {
        try!(face.load_char(ch as usize, ft::face::RENDER));

        let slot = face.glyph();
        try!(slot.render_glyph(ft::RenderMode::Normal));
        let bitmap = slot.bitmap();

        {
            use ft::bitmap::PixelMode::*;
            let mode = try!(bitmap.pixel_mode());
            match mode {
                Gray => (),
                _ => panic!("non-Gray ft pixel mode")
            }
        }

        let buf = bitmap.buffer();
        let pitch = bitmap.pitch();
        let w = bitmap.width();
        for y in 0..bitmap.rows() {
            for x in 0..w {
                let luma = buf[(y * pitch + x) as usize];
                img.put_pixel(x as u32 + cursor, y as u32, image::Luma([luma]));
            }
        }
        cursor += w as u32 + 1;
    }

    Ok(())
}


fn main() {
    let mut img = image::ImageBuffer::new(40, 20);

    blit(&mut img).unwrap();

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();
}
