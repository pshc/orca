use glium;
use image::ImageBuffer;

#[derive(Debug)]
struct PunchTray {
    texture: glium::Texture2d,
}

impl PunchTray {
    pub fn new(display: &glium::Display) -> Self {
        let pixel_fmt = glium::texture::UncompressedFloatFormat::U8;
        let mipmaps = glium::texture::MipmapsOption::NoMipmap;
        let size = 256;
        let texture = glium::Texture2d::empty_with_format(display, pixel_fmt, mipmaps,
                                                          size, size).unwrap();
        PunchTray {
            texture: texture,
        }
    }
}

struct Scene {
    props: Option<Props>,
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    pos: [f32; 2],
}

implement_vertex!(Vertex, pos);

#[derive(Debug)]
struct Props {
    program: glium::Program,
    i_buffer: glium::IndexBuffer<u8>,
    v_buffer: glium::VertexBuffer<Vertex>,
    tray: PunchTray,
}

impl Scene {
    fn new() -> Scene {
        Scene {props: None}
    }

    fn use_display(&mut self, display: &glium::Display) {
        let vsh = include_str!("vertex.glsl");
        let fsh = include_str!("fragment.glsl");
        let program = glium::Program::from_source(display, vsh, fsh, None).unwrap();

        let verts = vec![
            Vertex {pos: [-0.8, -0.8]},
            Vertex {pos: [ 0.8,  0.8]},
            Vertex {pos: [ 0.8, -0.8]},
            Vertex {pos: [-0.8,  0.8]},
        ];
        let v_buffer = glium::VertexBuffer::new(display, &verts).unwrap();
        let indices = vec![0, 1, 2, 0, 3, 1];
        let i_kind = glium::index::PrimitiveType::TrianglesList;
        let i_buffer = glium::IndexBuffer::new(display, i_kind, &indices).unwrap();

        self.props = Some(Props {
            program: program,
            v_buffer: v_buffer,
            i_buffer: i_buffer,
            tray: PunchTray::new(display),
        });
    }

    fn add_body(&mut self, body: &super::Body) {
        let p = match self.props {
            Some(ref props) => props,
            None => panic!("no display available")
        };
        let (w, h) = (200, 50);
        let mut img = ImageBuffer::new(w, h);
        super::draw_math(body, &mut img);
        let rect = glium::Rect {left: 0, bottom: 0, width: w, height: h};
        p.tray.texture.write(rect, img);
    }

    fn render<G: glium::Surface>(&self, gl: &mut G) {
        let p = match self.props {
            Some(ref props) => props,
            None => return
        };

        let ref uniforms = uniform! {
            tex: &p.tray.texture,
        };
        gl.draw(&p.v_buffer, &p.i_buffer, &p.program, uniforms, &Default::default()).unwrap();
    }
}

pub fn main() {
    use glium::DisplayBuild;
    let display = glium::glutin::WindowBuilder::new()
            .with_title(format!("orca"))
            .build_glium().unwrap();

    let ref mut scene = Scene::new();
    scene.use_display(&display);
    scene.add_body(&super::build_math());

    'main: loop {
        // out
        {
            let mut gl = display.draw();
            scene.render(&mut gl);
            gl.finish().unwrap();
        }

        // in
        for ev in display.poll_events() {
            use glium::glutin::Event::*;
            match ev {
                Closed => break 'main,
                ReceivedCharacter('q') => break 'main,
                _ => ()
            }
        }

        // chill
        {
            use std::thread::sleep_ms;
            sleep_ms(33);
        }
    }
}
