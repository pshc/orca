extern crate image;
#[macro_use]
extern crate log;

use std::fs::File;
use std::path::Path;


mod presses;

fn main() {
    let press = presses::FreeTypePress::new().unwrap();

    let mut img = image::ImageBuffer::new(40, 20);

    press.blit_str("Hello, world!", &mut img).unwrap();

    let ref mut fout = File::create(&Path::new("out.png")).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();
}
