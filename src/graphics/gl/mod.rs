extern crate glutin;
extern crate gl;
extern crate fps_counter;

use self::glutin::EventsLoop;
use self::glutin::GlContext;
use self::glutin::GlWindow;

use self::fps_counter::FPSCounter;

use std::mem;
use std::ptr;

use graphics::Renderer;
use graphics::RendererInfo;

use input::InputKey;

pub struct GLRenderer {
    gl_window : GlWindow,
    events_loop : EventsLoop,
    is_alive : bool,

    tex : u32,
    ebo : u32,
    vao : u32,
    vb : u32,
    program : u32,

    keys : Vec<self::glutin::VirtualKeyCode>,
    events_polled : bool,
    title : String,
    fps : FPSCounter

}

impl Drop for GLRenderer {
    fn drop(&mut self) {
        unsafe {
            self::gl::DeleteVertexArrays(1, &self.vao);
            self::gl::DeleteBuffers(2, [self.ebo, self.vb].as_ptr());
            self::gl::DeleteTextures(1, &self.tex);
            self::gl::DeleteProgram(self.program);
        }
    }
}

impl Renderer for GLRenderer {
    fn submit_frame(&mut self, frame : &[u8], width : usize, height : usize) {
        if !self.is_alive {
            return;
        }

        unsafe {
            self::gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            self::gl::Clear(self::gl::COLOR_BUFFER_BIT);

            self::gl::UseProgram(self.program);

            self::gl::ActiveTexture(self::gl::TEXTURE0);
            self::gl::BindTexture(self::gl::TEXTURE_2D, self.tex);

            self::gl::BindVertexArray(self.vao);
            self::gl::Uniform1i(self::gl::GetUniformLocation(self.program, b"tex\0".as_ptr() as *const _), 0);
            self::gl::TexParameteri(self::gl::TEXTURE_2D, self::gl::TEXTURE_WRAP_S, self::gl::CLAMP_TO_EDGE as self::gl::types::GLint);
            self::gl::TexParameteri(self::gl::TEXTURE_2D, self::gl::TEXTURE_WRAP_T, self::gl::CLAMP_TO_EDGE as self::gl::types::GLint);
            self::gl::TexParameteri(self::gl::TEXTURE_2D, self::gl::TEXTURE_MIN_FILTER, self::gl::NEAREST as self::gl::types::GLint);
            self::gl::TexParameteri(self::gl::TEXTURE_2D, self::gl::TEXTURE_MAG_FILTER, self::gl::NEAREST as self::gl::types::GLint);

            self::gl::TexImage2D(self::gl::TEXTURE_2D, 0, self::gl::RGB as self::gl::types::GLint,
                                 width as i32, height as i32, 0, self::gl::RGBA, self::gl::UNSIGNED_BYTE,
                                 frame.as_ptr() as *const _);

            self::gl::BindBuffer(self::gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            self::gl::DrawElements(self::gl::TRIANGLES, 6, self::gl::UNSIGNED_INT,
                                   (0 * mem::size_of::<f32>()) as *const () as *const _);
        }

        self.gl_window.swap_buffers().unwrap();
    }

    fn poll_events(&mut self) {
        if !self.is_alive {
            return;
        }

        self.events_polled = true;

        let mut events = Vec::new();
        self.events_loop.poll_events(|event| {
            events.push(event);
        });

        for event in events {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => self.is_alive = false,
                    glutin::WindowEvent::Resized(w, h) => {
                        self.gl_window.resize(w, h);
                        unsafe {
                            self::gl::Viewport(0, 0, w as _, h as _);
                        }
                    },
                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        match input.virtual_keycode {
                            Some(v) => {
                                if input.state == glutin::ElementState::Pressed
                                    && !self.keys.contains(&v) {
                                    self.keys.push(v);
                                } else if input.state == glutin::ElementState::Released {
                                    self.keys.remove_item(&v);
                                }
                            },
                            _ => {}
                        }
                    },
                    _ => ()
                }
                _ => ()
            }
        }
    }

    fn is_alive(&self) -> bool {
        self.is_alive
    }

    fn is_key_down(&self, key: &InputKey) -> bool {
        // Map input keys to glutin keys
        // TODO: this should be configurable

        let native_key = match key {
            &InputKey::A => self::glutin::VirtualKeyCode::A,
            &InputKey::B => self::glutin::VirtualKeyCode::S,
            &InputKey::X => self::glutin::VirtualKeyCode::Z,
            &InputKey::Y => self::glutin::VirtualKeyCode::X,
            &InputKey::Select => self::glutin::VirtualKeyCode::V,
            &InputKey::Start => self::glutin::VirtualKeyCode::B,
            &InputKey::Up => self::glutin::VirtualKeyCode::Up,
            &InputKey::Down => self::glutin::VirtualKeyCode::Down,
            &InputKey::Left => self::glutin::VirtualKeyCode::Left,
            &InputKey::Right => self::glutin::VirtualKeyCode::Right,
            &InputKey::L => self::glutin::VirtualKeyCode::Q,
            &InputKey::R => self::glutin::VirtualKeyCode::W,
            &InputKey::L2 => self::glutin::VirtualKeyCode::Key1,
            &InputKey::R2 => self::glutin::VirtualKeyCode::Key2,
            &InputKey::L3 => self::glutin::VirtualKeyCode::Key3,
            &InputKey::R3 => self::glutin::VirtualKeyCode::Key4
        };

        self.keys.contains(&native_key)
    }

    fn set_title(&mut self, title: String) {
        self.gl_window.set_title(&title);
        self.title = title;
    }
}

pub fn build(width : u32, height : u32) -> Box<Renderer> {
    let events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("OxRetro")
        .with_dimensions(width, height);
    let context = glutin::ContextBuilder::new()
        .with_vsync(true);
    let gl_window = glutin::GlWindow::new(window,
                                          context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
    }

    self::gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    let mut tex = unsafe { mem::uninitialized() };
    let mut ebo = unsafe { mem::uninitialized() };
    let mut vao;
    let mut vb;
    let program;

    unsafe {
        // Generate shaders
        // Stolen from https://github.com/tomaka/glutin/blob/master/examples/support/mod.rs &
        //             https://open.gl/content/code/c3_multitexture.txt
        let vs = self::gl::CreateShader(self::gl::VERTEX_SHADER);
        self::gl::ShaderSource(vs, 1, [VS_SRC.as_ptr() as *const _].as_ptr(), ptr::null());
        self::gl::CompileShader(vs);

        let fs = self::gl::CreateShader(self::gl::FRAGMENT_SHADER);
        self::gl::ShaderSource(fs, 1, [FS_SRC.as_ptr() as *const _].as_ptr(), ptr::null());
        self::gl::CompileShader(fs);

        program = self::gl::CreateProgram();
        self::gl::AttachShader(program, vs);
        self::gl::AttachShader(program, fs);
        self::gl::BindFragDataLocation(program, 0, b"outColor\0".as_ptr() as *const _);
        self::gl::LinkProgram(program);
        self::gl::UseProgram(program);

        self::gl::GenBuffers(1, &mut ebo);
        self::gl::BindBuffer(self::gl::ARRAY_BUFFER, ebo);
        self::gl::BufferData(self::gl::ARRAY_BUFFER,
                             (ELEMENTS.len() * mem::size_of::<u32>()) as self::gl::types::GLsizeiptr,
                             ELEMENTS.as_ptr() as *const _, self::gl::STATIC_DRAW);

        vb = mem::uninitialized();
        self::gl::GenBuffers(1, &mut vb);
        self::gl::BindBuffer(self::gl::ARRAY_BUFFER, vb);
        self::gl::BufferData(self::gl::ARRAY_BUFFER,
                             (VERTEX_DATA.len() * mem::size_of::<f32>()) as self::gl::types::GLsizeiptr,
                             VERTEX_DATA.as_ptr() as *const _, self::gl::STATIC_DRAW);

        vao = mem::uninitialized();
        self::gl::GenVertexArrays(1, &mut vao);
        self::gl::BindVertexArray(vao);

        let pos_attrib = self::gl::GetAttribLocation(program, b"position\0".as_ptr() as *const _);
        let color_attrib = self::gl::GetAttribLocation(program, b"color\0".as_ptr() as *const _);
        let tex_attrib = self::gl::GetAttribLocation(program, b"texcoord\0".as_ptr() as *const _);
        self::gl::VertexAttribPointer(pos_attrib as self::gl::types::GLuint, 2, self::gl::FLOAT, 0,
                                      7 * mem::size_of::<f32>() as self::gl::types::GLsizei,
                                      ptr::null());
        self::gl::VertexAttribPointer(color_attrib as self::gl::types::GLuint, 3, self::gl::FLOAT, 0,
                                      7 * mem::size_of::<f32>() as self::gl::types::GLsizei,
                                      (2 * mem::size_of::<f32>()) as *const () as *const _);
        self::gl::VertexAttribPointer(tex_attrib as self::gl::types::GLuint, 2, self::gl::FLOAT, 0,
                                      7 * mem::size_of::<f32>() as self::gl::types::GLsizei,
                                      (5 * mem::size_of::<f32>()) as *const () as *const _);
        self::gl::EnableVertexAttribArray(pos_attrib as self::gl::types::GLuint);
        self::gl::EnableVertexAttribArray(color_attrib as self::gl::types::GLuint);
        self::gl::EnableVertexAttribArray(tex_attrib as self::gl::types::GLuint);

        // Generate texture (for us to dump into)
        self::gl::GenTextures(1, &mut tex);
    }

    Box::new(
        GLRenderer {
            gl_window,
            events_loop,
            is_alive : true,

            tex,
            ebo,
            vao,
            vb,
            program,

            keys : Vec::new(),
            events_polled : true,
            title : "OxRetro".to_owned(),
            fps : FPSCounter::new()
        }
    )
}

pub static INFO : RendererInfo = RendererInfo {
    name: "OpenGL (Glutin)",
    provides_opengl: true,
    provides_vulkan: false,
};

// OpenGL resources
static VERTEX_DATA: [f32; 28] = [
    // X    Y    R    G    B    U    V
    -1.0,  1.0, 1.0, 1.0, 1.0, 0.0, 0.0, // Top-left
    1.0,  1.0, 1.0, 1.0, 1.0, 1.0, 0.0, // Top-right
    1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, // Bottom-right
    -1.0, -1.0, 1.0, 1.0, 1.0, 0.0, 1.0  // Bottom-left
];

static ELEMENTS: [u32; 6] = [
    0, 1, 2,
    2, 3, 0
];

const VS_SRC: &'static [u8] = b"
    #version 150 core

    in vec2 position;
    in vec3 color;
    in vec2 texcoord;

    out vec3 Color;
    out vec2 Texcoord;

    void main()
    {
        Color = color;
        Texcoord = texcoord;
        gl_Position = vec4(position, 0.0, 1.0);
    }
\0";

const FS_SRC: &'static [u8] = b"
    #version 150 core

    in vec3 Color;
    in vec2 Texcoord;

    out vec4 outColor;

    uniform sampler2D tex;

    void main()
    {
        outColor = texture(tex, Texcoord) * vec4(Color, 1.0);
    }
\0";