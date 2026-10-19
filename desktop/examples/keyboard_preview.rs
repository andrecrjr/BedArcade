#[cfg(target_os = "linux")]
use {egui_overlay as egui, gamepad_desktop::keyboard};
#[cfg(target_os = "linux")]
#[path = "../src/keyboard_view.rs"]
mod keyboard_view;

/// Render the real keyboard without a compositor or a connected controller.
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
    use std::io::Write;

    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/gamepad-keyboard-preview".to_owned());
    std::fs::create_dir_all(&directory)?;
    for (name, width, customizing) in [
        ("compact", 520, false),
        ("default", 640, false),
        ("large", 1000, false),
        ("settings", 640, true),
    ] {
        let mut session = keyboard::KeyboardSession::default();
        session.open = true;
        session.settings.width = width;
        session.customizing = customizing;
        let (width, height) = session.settings.surface_size();
        let context = egui::Context::default();
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width as f32, height as f32),
                )),
                ..Default::default()
            },
            |ui| keyboard_view::show(ui, &session),
        );
        let jobs = context.tessellate(output.shapes, output.pixels_per_point);
        let mut pixels = vec![[0u8; 4]; (width * height) as usize];
        let mut renderer = EguiSoftwareRender::new(ColorFieldOrder::Rgba);
        renderer.render(
            &mut BufferMutRef::new(&mut pixels, width as usize, height as usize),
            &jobs,
            &output.textures_delta,
            output.pixels_per_point,
        );
        let mut file =
            std::io::BufWriter::new(std::fs::File::create(format!("{directory}/{name}.ppm"))?);
        write!(file, "P6\n{width} {height}\n255\n")?;
        for pixel in pixels {
            file.write_all(&pixel[..3])?;
        }
        println!("{directory}/{name}.ppm");
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("A prévia em software é gerada no Linux; o layout é compartilhado com o Windows.");
}
