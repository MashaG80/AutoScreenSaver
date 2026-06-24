use std::fs;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;
use serde::Deserialize;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use fontdue::{Font, FontSettings};
use std::process::{Command, Child};
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default)]
struct Clock {
    time: String,
    date: String,
}

#[derive(Deserialize, Debug, Default)]
struct Sysinfo {
    cpu: f64,
    ram: f64,
}

#[derive(Deserialize, Debug, Default)]
struct Weather {
    city: Option<String>,
    temp: Option<f64>,
    description: Option<String>,
    icon: Option<String>,
    stale: bool,
    error: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct State {
    clock: Clock,
    sysinfo: Sysinfo,
    weather: Weather,
}

fn get_project_root() -> PathBuf {
    let exe_path = std::env::current_exe().expect("Failed to get exe path");
    // exe is at .../renderer/target/debug/renderer.exe (or .../release/...)
    exe_path
        .parent().unwrap()  // target/debug or target/release
        .parent().unwrap()  // target
        .parent().unwrap()  // renderer
        .parent().unwrap()  // oled-screensaver (project root)
        .to_path_buf()
}

fn read_state(project_root: &PathBuf) -> State {
    let path = project_root.join("data_daemon").join("state.json");
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => State::default(),
    }
}

fn draw_text(
    buffer: &mut [u32],
    buf_width: usize,
    buf_height: usize,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    px_size: f32,
    color: u32,
) {
    let mut cursor_x = x;

    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px_size);

        let glyph_x = cursor_x + metrics.xmin;
        let glyph_y = y - metrics.ymin - metrics.height as i32;

        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let alpha = bitmap[gy * metrics.width + gx] as f32 / 255.0;
                if alpha <= 0.0 {
                    continue;
                }

                let px = glyph_x + gx as i32;
                let py = glyph_y + gy as i32;

                if px < 0 || py < 0 || px as usize >= buf_width || py as usize >= buf_height {
                    continue;
                }

                let idx = py as usize * buf_width + px as usize;

                let out_r = (r * alpha) as u32;
                let out_g = (g * alpha) as u32;
                let out_b = (b * alpha) as u32;

                buffer[idx] = (out_r << 16) | (out_g << 8) | out_b;
            }
        }

        cursor_x += metrics.advance_width.round() as i32;
    }
}

struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    font: Font,
    start_time: Instant,
    python_process: Option<Child>,
    project_root: PathBuf,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Get all available monitors
        let monitors: Vec<_> = event_loop.available_monitors().collect();

        // Print them out for debugging (check terminal output)
        for (i, m) in monitors.iter().enumerate() {
            println!("Monitor {}: {:?} - {:?}", i, m.name(), m.size());
        }

        // Pick the second monitor if it exists, otherwise fall back to primary
        let target_monitor = monitors.get(1).cloned();

        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("OLED Screensaver")
                        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(target_monitor))),
                )
                .unwrap(),
        );

    let context = softbuffer::Context::new(window.clone()).unwrap();
    let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    self.window = Some(window);
    self.surface = Some(surface);
}

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { .. } => {
                if self.start_time.elapsed().as_secs_f32() > 1.0 {
                    event_loop.exit();
                }
            }

            //Rather not have mouse events myself uncomment if you want to test it with a mouse. The idea is to prevent accidental mouse movement from closing the screensaver.
            //WindowEvent::MouseInput { .. } => {
                //if self.start_time.elapsed().as_secs_f32() > 1.0 {
                    //event_loop.exit();
                //}
            //}
            //WindowEvent::CursorMoved { .. } => {
                //if self.start_time.elapsed().as_secs_f32() > 1.0 {
                    //event_loop.exit();
                //}
            //}

            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                let surface = self.surface.as_mut().unwrap();

                let size = window.inner_size();
                let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };

                surface.resize(width, height).unwrap();

                let mut buffer = surface.buffer_mut().unwrap();

                for pixel in buffer.iter_mut() {
                    *pixel = 0x00000000;
                }

                let state = read_state(&self.project_root);
                let w = size.width as usize;
                let h = size.height as usize;

                // --- Pixel-shift OLED protection ---
                let shift_period_secs = 60;
                let shift_amount = 500; 

                let elapsed_secs = self.start_time.elapsed().as_secs();
                let cycle = (elapsed_secs / shift_period_secs) % 4;

                let (offset_x, offset_y): (i32, i32) = match cycle {
                    0 => (0, 0),
                    1 => (shift_amount, 0),
                    2 => (shift_amount, shift_amount),
                    3 => (0, shift_amount),
                    _ => (0, 0),
                };

                draw_text(&mut buffer, w, h, &self.font, &state.clock.time, 60 + offset_x, 120 + offset_y, 80.0, 0xFFFFFF);
                draw_text(&mut buffer, w, h, &self.font, &state.clock.date, 60 + offset_x, 170 + offset_y, 32.0, 0xAAAAAA);

                if let Some(err) = &state.weather.error {
                    draw_text(&mut buffer, w, h, &self.font, err, 60 + offset_x, 260 + offset_y, 32.0, 0x884444);
                } else if !state.weather.stale {
                    let temp = state.weather.temp.unwrap_or(0.0);
                    let desc = state.weather.description.clone().unwrap_or_default();
                    let city = state.weather.city.clone().unwrap_or_default();

                    let weather_line = format!("{}°F  {}", temp, desc);
                    draw_text(&mut buffer, w, h, &self.font, &weather_line, 60 + offset_x, 260 + offset_y, 40.0, 0xFFFFFF);
                    draw_text(&mut buffer, w, h, &self.font, &city, 60 + offset_x, 300 + offset_y, 28.0, 0xAAAAAA);
                }

                let sys_line = format!("CPU: {:.1}%   RAM: {:.1}%", state.sysinfo.cpu, state.sysinfo.ram);
                draw_text(&mut buffer, w, h, &self.font, &sys_line, 60 + offset_x, 380 + offset_y, 28.0, 0x888888);

                buffer.present().unwrap();

                window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let project_root = get_project_root();
    let data_daemon_dir = project_root.join("data_daemon");
    let python_exe = data_daemon_dir.join(".venv").join("Scripts").join("python.exe");
    let script_path = data_daemon_dir.join("main.py");

    let python_process = Command::new(&python_exe)
        .arg(&script_path)
        .current_dir(&data_daemon_dir)
        .spawn()
        .expect("Failed to start data daemon");

    let font_bytes = fs::read(r"C:\Windows\Fonts\segoeui.ttf").expect("Failed to read font file");
    let font = Font::from_bytes(font_bytes, FontSettings::default()).expect("Failed to parse font");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App {
        window: None,
        surface: None,
        font,
        start_time: Instant::now(),
        python_process: Some(python_process),
        project_root,
    };

    event_loop.run_app(&mut app).unwrap();

    if let Some(mut child) = app.python_process.take() {
        let _ = child.kill();
    }
}