extern crate image;
#[macro_use]
extern crate log;

use std::fs::File;
use std::process::Command;


mod presses;

fn main() {
    let press = presses::FreeTypePress::new().unwrap();

    let mut img = image::ImageBuffer::new(40, 20);

    press.blit_str("Hello, world!", &mut img).unwrap();

    let filename = "out.png";
    let ref mut fout = File::create(filename).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();

    change_desktop_background(filename)
}

/// Invokes ./refresh for shoddy livecoding.
fn change_desktop_background(filename: &str) {
    if let Ok(mut cmd) = Command::new("sh").arg("-c").arg("./refresh").arg(filename).spawn() {
        println!("Refreshing.");
        let _ = cmd.wait();
    }
    else {
        println!("Wrote {}.", filename);
    }
}
