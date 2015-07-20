extern crate freetype;

use freetype::FtResult;

fn blit() -> FtResult<()> {
    let (ttf, pt, dpi) = ("test.ttf", 12, 96);

    let lib = try!(freetype::Library::init());
    let face = try!(lib.new_face(ttf, 0));
    try!(face.set_char_size(0, pt * 64, dpi, dpi));

    {
        let flags = freetype::face::LoadFlag::empty();
        try!(face.load_char('O' as usize, flags));

        let slot = face.glyph();
        try!(slot.render_glyph(freetype::RenderMode::Normal));
        let bitmap = slot.bitmap();

        {
            use freetype::bitmap::PixelMode::*;
            let mode = try!(bitmap.pixel_mode());
            match mode {
                Gray => (),
                _ => panic!("non-Gray ft pixel mode")
            }
        }

        let buf = bitmap.buffer();
        let pitch = bitmap.pitch();
        for y in 0..bitmap.rows() {
            for x in 0..bitmap.width() {
                let pixel = buf[(y * pitch + x) as usize];
                let c = match pixel {
                    0...63 => ' ',
                    64...127 => '.',
                    128...191 => 'o',
                    _ => 'O',
                };
                print!("{}", c);
            }
            println!("");
        }
    }

    Ok(())
}

fn main() {
    blit().unwrap()
}
