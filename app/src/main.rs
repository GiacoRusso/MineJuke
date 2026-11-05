//! Minecraft Jukebox - a native macOS music disc player.
//!
//! Run normally:
//!   cargo run --release
//!
//! Capture the rendered window (used by the build checker):
//!   cargo run --release -- --screenshot shots/now.png
//!
//! Drive the core loop headlessly and write a report:
//!   cargo run --release -- --self-test shots/self-test.json

mod app;
mod assets;
mod disc;
mod player;
mod selftest;
mod style;
mod textures;

use app::{JukeboxApp, Startup, MIN_WINDOW_H, MIN_WINDOW_W, WINDOW_H, WINDOW_W};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let mut screenshot = None;
    let mut self_test = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                screenshot = args.next().map(PathBuf::from);
                if screenshot.is_none() {
                    eprintln!("--screenshot needs a PNG path");
                    std::process::exit(2);
                }
            }
            "--self-test" => {
                self_test = args.next().map(PathBuf::from);
                if self_test.is_none() {
                    eprintln!("--self-test needs a JSON path");
                    std::process::exit(2);
                }
            }
            "--help" | "-h" => {
                println!("Minecraft Jukebox");
                println!("  --screenshot <file.png>   draw 120 frames, save a PNG and exit");
                println!("  --self-test <file.json>   run the headless core-loop test and exit");
                return Ok(());
            }
            _ => {}
        }
    }

    if let Some(path) = self_test {
        if !selftest::run(path) {
            std::process::exit(1);
        }
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Minecraft Jukebox")
            .with_inner_size([WINDOW_W, WINDOW_H])
            .with_min_inner_size([MIN_WINDOW_W, MIN_WINDOW_H])
            .with_resizable(true),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        multisampling: 0,
        ..Default::default()
    };

    let startup = Startup {
        screenshot,
        self_test: None,
    };

    eframe::run_native(
        "Minecraft Jukebox",
        options,
        Box::new(move |cc| Ok(Box::new(JukeboxApp::new(cc, startup)))),
    )
}
