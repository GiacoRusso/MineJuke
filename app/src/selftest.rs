//! Headless self-test used by Craft Anything.
//!
//! It drives the same state machine and renders the same egui UI as the real
//! app, then measures the result from the app's own fields and paint output.

use crate::app::{JukeboxApp, JukeboxState, Startup, WINDOW_H, WINDOW_W};
use crate::disc::DISCS;
use crate::player::Player;
use eframe::egui::{self, Event, Key, Modifiers, Pos2, RawInput, Rect, Vec2};
use serde_json::json;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> bool {
    let ctx = egui::Context::default();
    let startup = Startup {
        screenshot: None,
        self_test: Some(path.clone()),
    };
    let mut app = JukeboxApp::new_with_ctx(&ctx, startup);
    // The self-test is about the UI/state machine, not about the user's music
    // collection, so it uses the silent demo timer.
    app.setup_open = false;
    app.library = None;
    app.library_error = None;
    app.player = Player::simulated();
    app.demo_mode = true;
    app.repeat = false;
    app.config.repeat = false;

    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(WINDOW_W, WINDOW_H));

    // --- move: arrow key changes the selected disc ---------------------
    let before = app.selected;
    let raw = RawInput {
        screen_rect: Some(screen),
        events: vec![Event::Key {
            key: Key::ArrowRight,
            physical_key: Some(Key::ArrowRight),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| app.handle_input(ctx));
    let moved = app.selected != before;
    let move_detail = if moved {
        format!(
            "arrow key moved selection from {} to {}",
            DISCS[before].title, DISCS[app.selected].title
        )
    } else {
        "arrow key did not change the selected disc".to_string()
    };

    // --- act: space inserts the disc and playback advances -------------
    let raw = RawInput {
        screen_rect: Some(screen),
        events: vec![Event::Key {
            key: Key::Space,
            physical_key: Some(Key::Space),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| app.handle_input(ctx));
    for _ in 0..5 {
        app.advance(0.1);
    }
    for _ in 0..20 {
        app.advance(0.1);
    }
    let elapsed = app.player.elapsed().as_secs_f32();
    let act_ok = app.state == JukeboxState::Playing && elapsed >= 1.5;
    let act_detail = if act_ok {
        format!(
            "inserted {}; playback timer advanced {:.1} s",
            app.inserted.map(|i| DISCS[i].title).unwrap_or("disc"),
            elapsed
        )
    } else {
        format!(
            "state {:?}, playback timer {:.2} s (expected Playing and > 1.5 s)",
            app.state, elapsed
        )
    };

    // --- hud: render a real frame and look for the on-screen text ------
    let raw = RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let output = ctx.run(raw, |ctx| app.render(ctx));
    let has_now_playing = output
        .shapes
        .iter()
        .any(|s| shape_has_text(&s.shape, "NOW PLAYING"));
    let has_track = output
        .shapes
        .iter()
        .any(|s| shape_has_text(&s.shape, "C418 - Cat"));
    let has_controls = output
        .shapes
        .iter()
        .any(|s| shape_has_text(&s.shape, "REPEAT"))
        && output
            .shapes
            .iter()
            .any(|s| shape_has_text(&s.shape, "VOL"));
    let hud_ok = has_now_playing && has_track && has_controls;
    let hud_detail = if hud_ok {
        "now-playing line, progress controls and repeat/volume controls were drawn".to_string()
    } else {
        format!(
            "drawn text check: now_playing={has_now_playing}, track={has_track}, controls={has_controls}"
        )
    };

    let all_ok = moved && act_ok && hud_ok;
    let result = json!({
        "version": 1,
        "steps": [
            {"step": "move", "passed": moved, "detail": move_detail},
            {"step": "act", "passed": act_ok, "detail": act_detail},
            {"step": "hud", "passed": hud_ok, "detail": hud_detail}
        ]
    });

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!("Could not create self-test folder: {err}");
            return false;
        }
    }
    match std::fs::write(
        &path,
        serde_json::to_vec_pretty(&result).unwrap_or_default(),
    ) {
        Ok(()) => {
            eprintln!(
                "Self-test written to {} ({})",
                path.display(),
                if all_ok { "all steps passed" } else { "FAILED" }
            );
            all_ok
        }
        Err(err) => {
            eprintln!("Could not write self-test: {err}");
            false
        }
    }
}

fn shape_has_text(shape: &egui::Shape, needle: &str) -> bool {
    match shape {
        egui::Shape::Text(text) => text.galley.text().contains(needle),
        egui::Shape::Vec(shapes) => shapes.iter().any(|s| shape_has_text(s, needle)),
        _ => false,
    }
}
