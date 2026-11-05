//! The egui application: jukebox stage, disc inventory, animations and controls.

use crate::assets::{self, AssetLibrary};
use crate::disc::{self, DISCS};
use crate::player::Player;
use crate::style::{self, *};
use crate::textures::Textures;

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Id, Label, Layout, Margin, Order, Pos2, Rect,
    RichText, Sense, Stroke, Ui, Vec2, ViewportCommand,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const INSERT_SECS: f32 = 0.50;
pub const EJECT_SECS: f32 = 0.45;
/// Where the disc starts during the insert animation, as a fraction of the
/// jukebox art height below the centre. It rises from here into the playing
/// pose, so the hand-off from inserting to playing has no visible jump.
pub const INSERT_START_OFFSET: f32 = 0.45;
/// Starting disc size during the insert animation, as a fraction of the full
/// playing size.
pub const INSERT_START_SCALE: f32 = 0.18;
/// How far the disc pops up during the eject animation.
pub const EJECT_RISE: f32 = 0.32;
/// Idle bob while a track is playing (amplitude and angular speed).
pub const PLAYING_BOB_AMP: f32 = 2.0;
pub const PLAYING_BOB_SPEED: f32 = 4.0;
pub const SPIN_INSERT: f32 = 1.35;
pub const SPIN_PLAYING: f32 = 1.75;
pub const NOTE_INTERVAL: f32 = 0.28;
pub const NOTE_LIFE: f32 = 2.6;
pub const NOTE_RISE: f32 = 48.0;
pub const MAX_NOTES: usize = 40;
pub const DEFAULT_VOLUME: f32 = 0.70;
pub const WINDOW_W: f32 = 1180.0;
pub const WINDOW_H: f32 = 900.0;
pub const MIN_WINDOW_W: f32 = 900.0;
pub const MIN_WINDOW_H: f32 = 680.0;
pub const DEMO_TRACK_SECS: f32 = 185.0;
const CONFIG_SAVE_DELAY: f32 = 0.6;
const TOAST_SECS: f32 = 3.8;

#[derive(Clone, Default)]
pub struct Startup {
    pub screenshot: Option<PathBuf>,
    pub self_test: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JukeboxState {
    Idle,
    Inserting,
    Playing,
    Ejecting,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub minecraft_dir: Option<String>,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub repeat: bool,
}

fn default_volume() -> f32 {
    DEFAULT_VOLUME
}

impl Default for Config {
    fn default() -> Self {
        Self {
            minecraft_dir: None,
            volume: DEFAULT_VOLUME,
            repeat: false,
        }
    }
}

impl Config {
    fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .unwrap_or_default()
    }
}

struct Note {
    x: f32,
    y: f32,
    vy: f32,
    sway: f32,
    phase: f32,
    life: f32,
    max_life: f32,
    color: Color32,
    kind: u8,
}

pub struct JukeboxApp {
    pub config: Config,
    config_path: PathBuf,
    pub library: Option<AssetLibrary>,
    pub library_error: Option<String>,
    pub setup_open: bool,
    pub setup_error: Option<String>,
    pub textures: Textures,

    pub selected: usize,
    pub inserted: Option<usize>,
    pub state: JukeboxState,
    pub player: Player,
    pub volume: f32,
    pub repeat: bool,

    anim_t: f32,
    anim_time: f32,
    spin_angle: f32,
    notes: Vec<Note>,
    note_accum: f32,
    rng: u32,
    jukebox_rect: Rect,
    drag_disc: Option<usize>,
    drag_pos: Option<Pos2>,

    toast: Option<(String, f32)>,
    screenshot_path: Option<PathBuf>,
    screenshot_requested: bool,
    frame_count: u64,
    demo_started: bool,
    pub demo_mode: bool,

    setup_default_available: bool,
    config_dirty: bool,
    save_timer: f32,
}

impl JukeboxApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        Self::new_with_ctx(&cc.egui_ctx, startup)
    }

    /// Also used by the headless self-test, so the UI is the same code path.
    pub fn new_with_ctx(ctx: &egui::Context, startup: Startup) -> Self {
        style::install_fonts(ctx);
        style::apply_visuals(ctx);
        let textures = Textures::new(ctx);

        let config_path = config_path();
        let config = Config::load(&config_path);
        let screenshot_mode = startup.screenshot.is_some();
        let self_test_mode = startup.self_test.is_some();
        let demo_mode = screenshot_mode || self_test_mode;

        let mut library = None;
        let mut library_error = None;
        if let Some(dir) = config.minecraft_dir.as_ref() {
            match AssetLibrary::load(Path::new(dir)) {
                Ok(lib) => library = Some(lib),
                Err(err) => {
                    library_error =
                        Some(format!("Could not read the saved Minecraft folder: {err}"));
                }
            }
        }

        // In screenshot mode we still want a representative header/banner even
        // when the app has never been configured. This never writes config.
        if screenshot_mode && library.is_none() {
            let default = assets::default_minecraft_dir();
            if let Ok(lib) = AssetLibrary::load(&default) {
                library = Some(lib);
            }
        }

        let default_dir = assets::default_minecraft_dir();
        let setup_default_available =
            default_dir.join("assets/indexes").is_dir() || default_dir.join("indexes").is_dir();
        let setup_open = if demo_mode {
            false
        } else {
            config.minecraft_dir.is_none()
                || library
                    .as_ref()
                    .map(|lib| lib.found_count() == 0)
                    .unwrap_or(true)
        };
        let setup_error = if !demo_mode && config.minecraft_dir.is_some() && library.is_none() {
            library_error.clone()
        } else {
            None
        };

        let mut player = if demo_mode {
            Player::simulated()
        } else {
            Player::new()
        };
        let volume = config.volume.clamp(0.0, 1.0);
        let repeat = config.repeat;
        player.set_volume(volume);

        Self {
            config,
            config_path,
            library,
            library_error,
            setup_open,
            setup_error,
            textures,
            selected: 0,
            inserted: None,
            state: JukeboxState::Idle,
            player,
            volume,
            repeat,
            anim_t: 0.0,
            anim_time: 0.0,
            spin_angle: 0.0,
            notes: Vec::new(),
            note_accum: 0.0,
            rng: 0x5EED_1234,
            jukebox_rect: Rect::NOTHING,
            drag_disc: None,
            drag_pos: None,
            toast: None,
            screenshot_path: startup.screenshot,
            screenshot_requested: false,
            frame_count: 0,
            demo_started: false,
            demo_mode,
            setup_default_available,
            config_dirty: false,
            save_timer: 0.0,
        }
    }

    fn config_path_display(&self) -> String {
        assets::display_path(&self.config_path)
    }

    pub fn save_config(&mut self) {
        self.config.volume = self.volume;
        self.config.repeat = self.repeat;
        if let Some(parent) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&self.config) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&self.config_path, json) {
                    eprintln!("Could not save config: {err}");
                }
            }
            Err(err) => eprintln!("Could not serialize config: {err}"),
        }
        self.config_dirty = false;
        self.save_timer = 0.0;
    }

    pub fn set_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), TOAST_SECS));
    }

    // ------------------------------------------------------------------
    // Playback state machine
    // ------------------------------------------------------------------

    pub fn begin_insert(&mut self, index: usize) {
        if index >= DISCS.len() {
            return;
        }
        if self.demo_mode {
            self.begin_insert_demo(index);
            return;
        }

        let disc = DISCS[index];
        let path = match self.library.as_ref() {
            Some(lib) if lib.found_count() == 0 => {
                self.set_toast("Choose your Minecraft folder first.");
                self.setup_open = true;
                return;
            }
            Some(lib) => match lib.record_path(&disc) {
                Some(path) => path.to_path_buf(),
                None => {
                    self.set_toast(format!(
                        "{} isn't in this Minecraft version. Try another disc.",
                        disc.title
                    ));
                    return;
                }
            },
            None => {
                self.set_toast("Choose your Minecraft folder first.");
                self.setup_open = true;
                return;
            }
        };

        match self.player.play(&path) {
            Ok(_) => {
                self.inserted = Some(index);
                self.selected = index;
                self.state = JukeboxState::Inserting;
                self.anim_t = 0.0;
                self.spin_angle = 0.0;
                self.notes.clear();
                self.note_accum = 0.0;
                if let Some(err) = self.player.audio_error.take() {
                    self.set_toast(format!("Playing without sound: {err}"));
                }
            }
            Err(err) => {
                self.player.stop();
                self.state = JukeboxState::Idle;
                self.inserted = None;
                self.set_toast(format!("Couldn't play {}: {err}", disc.title));
            }
        }
    }

    fn begin_insert_demo(&mut self, index: usize) {
        self.player
            .play_simulated(Duration::from_secs_f32(DEMO_TRACK_SECS));
        self.inserted = Some(index);
        self.selected = index;
        self.state = JukeboxState::Inserting;
        self.anim_t = 0.0;
        self.spin_angle = 0.0;
        self.notes.clear();
        self.note_accum = 0.0;
    }

    pub fn eject(&mut self) {
        if matches!(self.state, JukeboxState::Idle | JukeboxState::Ejecting) {
            return;
        }
        self.player.stop();
        self.state = JukeboxState::Ejecting;
        self.anim_t = 0.0;
        self.notes.clear();
        self.note_accum = 0.0;
    }

    pub fn toggle_play_eject(&mut self) {
        if self.setup_open {
            return;
        }
        match self.state {
            JukeboxState::Idle => self.begin_insert(self.selected),
            _ => self.eject(),
        }
    }

    pub fn change_disc(&mut self, delta: isize) {
        let n = DISCS.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(n) as usize;
        if matches!(self.state, JukeboxState::Inserting | JukeboxState::Playing)
            && self.inserted != Some(self.selected)
        {
            self.begin_insert(self.selected);
        }
    }

    fn replay_current(&mut self) {
        let Some(index) = self.inserted else {
            self.eject();
            return;
        };
        if self.demo_mode {
            self.player
                .play_simulated(Duration::from_secs_f32(DEMO_TRACK_SECS));
            self.state = JukeboxState::Playing;
            self.anim_t = 0.0;
            self.notes.clear();
            self.note_accum = 0.0;
            return;
        }

        let disc = DISCS[index];
        let path = self
            .library
            .as_ref()
            .and_then(|lib| lib.record_path(&disc))
            .map(|p| p.to_path_buf());
        let Some(path) = path else {
            self.eject();
            return;
        };
        match self.player.play(&path) {
            Ok(_) => {
                self.state = JukeboxState::Playing;
                self.anim_t = 0.0;
                self.notes.clear();
                self.note_accum = 0.0;
            }
            Err(err) => {
                self.set_toast(format!("Couldn't repeat {}: {err}", disc.title));
                self.eject();
            }
        }
    }

    fn seek_to(&mut self, ratio: f32) {
        let Some(duration) = self.player.duration() else {
            return;
        };
        let position = duration.mul_f32(ratio.clamp(0.0, 1.0));
        self.player.seek(position);
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        let (space, left, right, up, down, enter, escape) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });

        if escape && self.setup_open {
            self.setup_open = false;
            return;
        }
        if self.setup_open {
            return;
        }
        if space {
            self.toggle_play_eject();
        } else if enter {
            self.begin_insert(self.selected);
        }
        if left || up {
            self.change_disc(-1);
        }
        if right || down {
            self.change_disc(1);
        }
    }

    pub fn advance(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.anim_time += dt;

        if let Some((_, t)) = &mut self.toast {
            *t -= dt;
            if *t <= 0.0 {
                self.toast = None;
            }
        }
        if self.config_dirty {
            self.save_timer += dt;
            if self.save_timer >= CONFIG_SAVE_DELAY {
                self.save_config();
            }
        }

        match self.state {
            JukeboxState::Inserting => {
                self.anim_t += dt / INSERT_SECS;
                self.spin_angle += dt * SPIN_INSERT;
                if self.anim_t >= 1.0 {
                    self.anim_t = 1.0;
                    self.state = JukeboxState::Playing;
                    self.anim_t = 0.0;
                    if let Some(err) = self.player.audio_error.take() {
                        self.set_toast(format!("Playing without sound: {err}"));
                    }
                }
            }
            JukeboxState::Playing => {
                self.player.advance(Duration::from_secs_f32(dt));
                self.spin_angle += dt * SPIN_PLAYING;
                self.spawn_notes(dt);
                if self.player.is_finished() {
                    if self.repeat {
                        self.replay_current();
                    } else {
                        self.eject();
                    }
                }
            }
            JukeboxState::Ejecting => {
                self.anim_t += dt / EJECT_SECS;
                if self.anim_t >= 1.0 {
                    self.anim_t = 1.0;
                    self.state = JukeboxState::Idle;
                    self.inserted = None;
                    self.anim_t = 0.0;
                }
            }
            JukeboxState::Idle => {}
        }

        for note in &mut self.notes {
            note.life -= dt;
            note.y -= note.vy * dt;
            note.x += (note.phase + self.anim_time * 3.2).sin() * note.sway * dt;
        }
        self.notes.retain(|n| n.life > 0.0);
    }

    fn next_random(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn spawn_notes(&mut self, dt: f32) {
        self.note_accum += dt;
        while self.note_accum >= NOTE_INTERVAL {
            self.note_accum -= NOTE_INTERVAL;
            if self.notes.len() >= MAX_NOTES {
                continue;
            }
            let rect = if self.jukebox_rect.is_positive() {
                self.jukebox_rect
            } else {
                Rect::from_center_size(Pos2::new(590.0, 420.0), Vec2::splat(250.0))
            };
            let r1 = self.next_random();
            let r2 = self.next_random();
            let r3 = self.next_random();
            let r4 = self.next_random();
            let disc = self.inserted.map(|i| DISCS[i]).unwrap_or(DISCS[0]);
            let palette = [GOLD, CYAN, MAGENTA, GREEN, ORANGE, PURPLE, TEXT];
            let base = palette[(r4 * palette.len() as f32) as usize % palette.len()];
            let accent = Color32::from_rgb(disc.accent.0, disc.accent.1, disc.accent.2);
            let color = if r3 > 0.55 { accent } else { base };
            let x = rect.center().x + (r1 - 0.5) * rect.width() * 0.62;
            let y = rect.top() + rect.height() * (0.28 + r2 * 0.10);
            let life = NOTE_LIFE * (0.80 + r3 * 0.35);
            self.notes.push(Note {
                x,
                y,
                vy: NOTE_RISE * (0.85 + r4 * 0.45),
                sway: 5.0 + r1 * 9.0,
                phase: r2 * std::f32::consts::TAU,
                life,
                max_life: life,
                color,
                kind: (r4 * 4.0) as u8,
            });
        }
    }

    // ------------------------------------------------------------------
    // Folder selection
    // ------------------------------------------------------------------

    fn try_set_folder(&mut self, path: PathBuf) {
        match AssetLibrary::load(&path) {
            Ok(lib) => {
                let found = lib.found_count();
                self.library = Some(lib);
                self.library_error = None;
                self.setup_error = None;
                if found == 0 {
                    // Do not remember a folder that has no playable discs. A
                    // later volume/repeat save must not overwrite a working
                    // path with this bad one either.
                    self.setup_open = true;
                    self.setup_error = Some(format!(
                        "That folder has Minecraft assets, but none of the {} music disc tracks were found. Choose the folder that contains 'assets'.",
                        DISCS.len()
                    ));
                } else {
                    self.config.minecraft_dir = Some(path.to_string_lossy().into_owned());
                    self.save_config();
                    self.setup_open = false;
                    self.set_toast(format!("Loaded {found} of {} music discs.", DISCS.len()));
                }
            }
            Err(err) => {
                self.setup_error = Some(format!(
                    "That doesn't look like a Minecraft Java folder. {err}"
                ));
            }
        }
    }

    fn pick_folder(&mut self) {
        let start = self
            .config
            .minecraft_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(assets::default_minecraft_dir);
        let mut dialog = rfd::FileDialog::new().set_title("Choose your Minecraft Java folder");
        if start.is_dir() {
            dialog = dialog.set_directory(&start);
        }
        if let Some(path) = dialog.pick_folder() {
            self.try_set_folder(path);
        }
    }

    fn use_default_folder(&mut self) {
        let path = assets::default_minecraft_dir();
        if path.is_dir() {
            self.try_set_folder(path);
        } else {
            self.setup_error = Some(format!(
                "The default folder was not found at {}. Use Choose Folder instead.",
                assets::display_path(&path)
            ));
        }
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    pub fn render(&mut self, ctx: &egui::Context) {
        self.paint_background(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(Margin::symmetric(16.0, 12.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_enabled_ui(!self.setup_open, |ui| {
                            self.main_content(ui);
                        });
                    });
            });

        if self.setup_open {
            self.setup_modal(ctx);
        }
        self.paint_drag(ctx);
        self.paint_toast(ctx);
    }

    fn paint_background(&self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        let uv = Rect::from_min_size(Pos2::ZERO, screen.size() / 64.0);
        painter.image(
            self.textures.background.id(),
            screen,
            uv,
            Color32::from_rgb(138, 126, 110),
        );
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(7, 9, 13, 112));
        // darken the edges a little so the inventory panels pop.
        for i in 0..10 {
            let a = (10 - i) as u8 * 3;
            let h = 40.0;
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(0.0, i as f32 * 4.0), Vec2::new(screen.width(), h)),
                0.0,
                Color32::from_black_alpha(a),
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(0.0, screen.bottom() - (i as f32 * 4.0) - h),
                    Vec2::new(screen.width(), h),
                ),
                0.0,
                Color32::from_black_alpha(a),
            );
        }
    }

    fn main_content(&mut self, ui: &mut Ui) {
        let width = ui.available_width();
        self.header(ui);

        let found = self
            .library
            .as_ref()
            .map(|lib| lib.found_count())
            .unwrap_or(0);
        let missing = self
            .library
            .as_ref()
            .map(|lib| lib.found_count() < lib.total_count())
            .unwrap_or(true);
        if found == 0 {
            ui.add_space(10.0);
            self.banner(ui, width, true);
        } else if missing {
            ui.add_space(10.0);
            self.banner(ui, width, false);
        }

        ui.add_space(8.0);
        self.jukebox_panel(ui, width);
        ui.add_space(14.0);
        self.disc_panel(ui, width);
        ui.add_space(8.0);
        self.footer(ui);
    }

    fn header(&mut self, ui: &mut Ui) {
        let desired = Vec2::new(ui.available_width(), 60.0);
        let rect = Rect::from_min_size(ui.cursor().min, desired);
        let uv = Rect::from_min_size(Pos2::ZERO, desired / 32.0);
        ui.painter().image(
            self.textures.wood.id(),
            rect,
            uv,
            Color32::from_rgb(158, 134, 110),
        );
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(8, 10, 14, 78));
        style::paint_bevel(ui.painter(), rect.shrink(2.0), 2.0);

        ui.allocate_ui_with_layout(desired, Layout::left_to_right(Align::Center), |ui| {
            ui.add_space(12.0);
            let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            style::draw_note_icon(ui.painter(), icon_rect.center(), 28.0, GOLD);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(1.0);
                style::pixel_label(ui, "MINECRAFT JUKEBOX", 15.0, GOLD);
                style::pixel_label(ui, "MUSIC DISC PLAYER", 6.0, MUTED);
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                if style::mc_button(ui, "FOLDER", Vec2::new(88.0, 30.0), true).clicked() {
                    self.setup_open = true;
                    self.setup_error = None;
                }
                ui.add_space(8.0);
                match &self.library {
                    Some(lib) if lib.found_count() > 0 => {
                        style::status_pill(ui, "MUSIC READY", GREEN);
                    }
                    Some(_) => {
                        style::status_pill(ui, "NO TRACKS", RED);
                    }
                    None => {
                        style::status_pill(ui, "FOLDER NOT SET", RED);
                    }
                }
            });
        });
    }

    fn banner(&mut self, ui: &mut Ui, width: f32, no_folder: bool) {
        let bg = if no_folder {
            Color32::from_rgb(53, 28, 26)
        } else {
            Color32::from_rgb(51, 40, 22)
        };
        let border = if no_folder { RED } else { AMBER };
        let title = if no_folder {
            "MUSIC FILES NOT FOUND"
        } else {
            "SOME DISCS ARE MISSING"
        };
        let body = if no_folder {
            "Choose your Minecraft Java folder to load the music already installed on this Mac."
        } else {
            "This Minecraft version doesn't include every disc. The rest are ready to play."
        };

        let frame = egui::Frame::none()
            .fill(bg)
            .stroke(Stroke::new(2.0_f32, border.gamma_multiply(0.65)))
            .inner_margin(Margin::symmetric(12.0, 8.0));
        let mut clicked = false;
        let response = frame.show(ui, |ui| {
            ui.set_width((width - 28.0).max(200.0));
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                style::pixel_text(
                    ui.painter(),
                    icon_rect.center(),
                    Align2::CENTER_CENTER,
                    "!",
                    18.0,
                    border,
                );
                ui.vertical(|ui| {
                    style::pixel_label(ui, title, 8.0, border);
                    ui.add(Label::new(RichText::new(body).size(7.0).color(TEXT)).wrap());
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if style::mc_button(ui, "CHOOSE FOLDER", Vec2::new(156.0, 30.0), true).clicked()
                    {
                        clicked = true;
                    }
                });
            });
        });
        style::paint_bevel(ui.painter(), response.response.rect.shrink(2.0), 2.0);
        if clicked {
            self.setup_open = true;
            self.setup_error = None;
        }
    }

    fn jukebox_panel(&mut self, ui: &mut Ui, avail: f32) {
        let panel_w = avail.clamp(620.0, 840.0);
        let frame = egui::Frame::none()
            .fill(PANEL)
            .stroke(Stroke::new(2.0_f32, OUTLINE))
            .inner_margin(Margin::symmetric(16.0, 14.0));

        let outer = ui.vertical_centered(|ui| {
            ui.set_max_width(panel_w);
            frame.show(ui, |ui| {
                let inner_w = (panel_w - 32.0).max(400.0);
                ui.set_width(inner_w);

                ui.horizontal(|ui| {
                    style::pixel_label(ui, "JUKEBOX", 10.0, GOLD);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        style::pixel_label(ui, "BLOCK 84", 6.0, MUTED_DARK);
                    });
                });

                let art_size = 250.0_f32.min(inner_w * 0.42).max(180.0);
                let button_w = 50.0;
                let row_w = button_w * 2.0 + art_size + 40.0;
                ui.horizontal(|ui| {
                    ui.add_space(((inner_w - row_w) / 2.0).max(0.0));
                    if style::mc_button(ui, "<", Vec2::new(button_w, 52.0), true).clicked() {
                        self.change_disc(-1);
                    }
                    ui.add_space(20.0);
                    let (art_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(art_size), Sense::hover());
                    self.jukebox_rect = art_rect;
                    self.draw_jukebox(ui.painter(), art_rect);
                    ui.add_space(20.0);
                    if style::mc_button(ui, ">", Vec2::new(button_w, 52.0), true).clicked() {
                        self.change_disc(1);
                    }
                });

                let playing = matches!(
                    self.state,
                    JukeboxState::Inserting | JukeboxState::Playing | JukeboxState::Ejecting
                ) && self.inserted.is_some();

                ui.add_space(2.0);
                ui.vertical_centered(|ui| {
                    if playing {
                        style::pixel_label(ui, "NOW PLAYING:", 7.0, GOLD);
                        if let Some(index) = self.inserted {
                            let disc = DISCS[index];
                            style::pixel_label(
                                ui,
                                &format!("{} - {}", disc.artist, disc.title),
                                11.0,
                                TEXT,
                            );
                        }
                    } else {
                        style::pixel_label(ui, "JUKEBOX EMPTY", 7.0, MUTED);
                        style::pixel_label(ui, "DOUBLE-CLICK A DISC BELOW", 7.0, MUTED);
                    }
                });

                ui.add_space(7.0);
                let elapsed = self.player.elapsed().as_secs_f32();
                let duration = self
                    .player
                    .duration()
                    .map(|d| d.as_secs_f32())
                    .unwrap_or(0.0);
                let ratio = if duration > 0.0 {
                    (elapsed / duration).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [46.0, 18.0],
                        Label::new(
                            RichText::new(disc::format_time(elapsed))
                                .size(7.0)
                                .color(MUTED),
                        ),
                    );
                    let bar_w = (ui.available_width() - 104.0).max(100.0);
                    let (_, seek) = style::progress_bar(ui, ratio, bar_w);
                    if let Some(ratio) = seek {
                        self.seek_to(ratio);
                    }
                    ui.add_sized(
                        [46.0, 18.0],
                        Label::new(
                            RichText::new(if duration > 0.0 {
                                disc::format_time(duration)
                            } else {
                                "--:--".to_string()
                            })
                            .size(7.0)
                            .color(MUTED),
                        ),
                    );
                });

                ui.add_space(3.0);
                let controls_w = 100.0 + 8.0 + 110.0 + 12.0 + 30.0 + 8.0 + 150.0 + 8.0 + 44.0;
                ui.horizontal(|ui| {
                    ui.add_space(((inner_w - controls_w) / 2.0).max(0.0));
                    let can_eject = matches!(
                        self.state,
                        JukeboxState::Inserting | JukeboxState::Playing | JukeboxState::Ejecting
                    );
                    if style::mc_button(ui, "EJECT", Vec2::new(100.0, 32.0), can_eject).clicked() {
                        self.eject();
                    }
                    ui.add_space(8.0);
                    if style::toggle_button(
                        ui,
                        if self.repeat {
                            "REPEAT ON"
                        } else {
                            "REPEAT OFF"
                        },
                        Vec2::new(110.0, 32.0),
                        true,
                        self.repeat,
                    )
                    .clicked()
                    {
                        self.repeat = !self.repeat;
                        self.config.repeat = self.repeat;
                        self.config_dirty = true;
                    }
                    ui.add_space(12.0);
                    style::pixel_label(ui, "VOL", 6.0, MUTED);
                    let (_, changed) = style::slider(ui, &mut self.volume, 150.0);
                    if changed {
                        self.player.set_volume(self.volume);
                        self.config.volume = self.volume;
                        self.config_dirty = true;
                    }
                    style::pixel_label(
                        ui,
                        &format!("{:>3}%", (self.volume * 100.0).round() as i32),
                        6.0,
                        TEXT,
                    );
                });
                ui.add_space(3.0);
                ui.vertical_centered(|ui| {
                    style::pixel_label(
                        ui,
                        "SPACE: PLAY / EJECT    < >: CHANGE DISC",
                        6.0,
                        MUTED_DARK,
                    );
                });
            });
        });

        style::paint_bevel(ui.painter(), outer.response.rect.shrink(2.0), 2.0);
        ui.painter().rect_stroke(
            outer.response.rect.shrink(5.0),
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(50, 54, 62)),
        );
    }

    fn draw_jukebox(&self, painter: &egui::Painter, rect: Rect) {
        let glow = matches!(self.state, JukeboxState::Inserting | JukeboxState::Playing)
            && self.inserted.is_some();

        if glow {
            for i in 0..4 {
                let expand = 3.0 + i as f32 * 4.0;
                painter.rect_filled(
                    rect.expand(expand),
                    8.0,
                    Color32::from_rgba_unmultiplied(255, 205, 90, 12 - i * 2),
                );
            }
        }

        painter.rect_filled(
            Rect::from_min_size(rect.min + Vec2::new(8.0, 10.0), rect.size()),
            0.0,
            Color32::from_black_alpha(120),
        );
        painter.image(
            self.textures.jukebox.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        let slot = Rect::from_min_max(
            rect.min + Vec2::new(rect.width() * 0.25, rect.height() * 0.11),
            rect.min + Vec2::new(rect.width() * 0.75, rect.height() * 0.28),
        );
        painter.rect_filled(slot, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 130));

        if let Some(index) = self.inserted {
            let slot_size = rect.width() * 0.40;
            let slot_center = slot.center();
            let (center, size, alpha, spinning) = match self.state {
                JukeboxState::Inserting => {
                    // Rise and grow from below the block straight into the
                    // exact pose used while playing. The previous animation
                    // ended on a small disc near the slot and then snapped to
                    // the full-size platter; matching the endpoints makes the
                    // transition continuous.
                    let t = self.anim_t.clamp(0.0, 1.0);
                    let eased = ease_in_out(t);
                    let bob = (self.anim_time * PLAYING_BOB_SPEED).sin()
                        * PLAYING_BOB_AMP
                        * eased;
                    let start = rect.center() + Vec2::new(0.0, rect.height() * INSERT_START_OFFSET);
                    let end = rect.center() + Vec2::new(0.0, bob);
                    (
                        Pos2::new(
                            start.x + (end.x - start.x) * eased,
                            start.y + (end.y - start.y) * eased,
                        ),
                        rect.width() * (INSERT_START_SCALE + (1.0 - INSERT_START_SCALE) * eased),
                        (eased * 255.0) as u8,
                        true,
                    )
                }
                JukeboxState::Ejecting => {
                    // Start from the full playing pose, then shrink, rise and
                    // fade out so ejecting does not snap either.
                    let t = self.anim_t.clamp(0.0, 1.0);
                    let eased = ease_in_out(t);
                    (
                        rect.center() + Vec2::new(0.0, -eased * rect.height() * EJECT_RISE),
                        rect.width() + (slot_size - rect.width()) * eased,
                        ((1.0 - t) * 255.0) as u8,
                        false,
                    )
                }
                JukeboxState::Playing => {
                    // While the track plays, the record grows to fill the whole
                    // jukebox block, like a turntable platter seen from above.
                    let bob = (self.anim_time * PLAYING_BOB_SPEED).sin() * PLAYING_BOB_AMP;
                    (rect.center() + Vec2::new(0.0, bob), rect.width(), 255, true)
                }
                JukeboxState::Idle => (slot_center, slot_size, 255, false),
            };
            let disc_rect = Rect::from_center_size(center, Vec2::splat(size));
            let tint = Color32::from_white_alpha(alpha);
            let tex = self.textures.discs[index].id();
            if spinning {
                self.paint_spinning_disc(painter, tex, center, size * 0.5, self.spin_angle, tint);
            } else {
                painter.image(
                    tex,
                    disc_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    tint,
                );
            }

            if matches!(self.state, JukeboxState::Inserting) {
                let t = self.anim_t.clamp(0.0, 1.0);
                for i in 0..7 {
                    let angle = i as f32 * 0.9 + self.anim_time * 5.0;
                    let radius = 18.0 + (1.0 - t) * 34.0 + (i % 3) as f32 * 7.0;
                    let p = center + Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.55);
                    painter.rect_filled(
                        Rect::from_center_size(p, Vec2::splat(4.0)),
                        0.0,
                        Color32::from_rgba_unmultiplied(255, 220, 120, (t * 220.0) as u8),
                    );
                }
            }
        }

        for note in &self.notes {
            let alpha = ((note.life / note.max_life).clamp(0.0, 1.0) * 255.0) as u8;
            let color = Color32::from_rgba_unmultiplied(
                note.color.r(),
                note.color.g(),
                note.color.b(),
                alpha,
            );
            style::draw_music_note(painter, Pos2::new(note.x, note.y), 18.0, color, note.kind);
        }
    }

    fn paint_spinning_disc(
        &self,
        painter: &egui::Painter,
        tex: egui::TextureId,
        center: Pos2,
        half: f32,
        angle: f32,
        tint: Color32,
    ) {
        let (s, c) = angle.sin_cos();
        let rot = |x: f32, y: f32| -> Pos2 {
            Pos2::new(center.x + x * c - y * s, center.y + x * s + y * c)
        };
        let mut mesh = egui::epaint::Mesh::with_texture(tex);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: rot(-half, -half),
            uv: Pos2::new(0.0, 0.0),
            color: tint,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: rot(half, -half),
            uv: Pos2::new(1.0, 0.0),
            color: tint,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: rot(half, half),
            uv: Pos2::new(1.0, 1.0),
            color: tint,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: rot(-half, half),
            uv: Pos2::new(0.0, 1.0),
            color: tint,
        });
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        painter.add(egui::Shape::mesh(mesh));
    }

    fn disc_panel(&mut self, ui: &mut Ui, avail: f32) {
        let panel_w = avail.clamp(620.0, 1120.0);
        let frame = egui::Frame::none()
            .fill(PANEL)
            .stroke(Stroke::new(2.0_f32, OUTLINE))
            .inner_margin(Margin::symmetric(14.0, 12.0));

        let mut clicked_select = None;
        let mut double_click = None;
        let mut drag_start = None;
        let mut drag_update = None;
        let mut drag_stop = None;

        let outer = ui.vertical_centered(|ui| {
            ui.set_max_width(panel_w);
            frame.show(ui, |ui| {
                let inner_w = (panel_w - 28.0).max(400.0);
                ui.set_width(inner_w);
                ui.horizontal(|ui| {
                    style::pixel_label(ui, "MUSIC DISCS", 10.0, GOLD);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let ready = self.library.as_ref().map(|l| l.found_count()).unwrap_or(0);
                        style::pixel_label(
                            ui,
                            &format!("{ready} / {} READY", DISCS.len()),
                            6.0,
                            if ready == DISCS.len() { GREEN } else { MUTED },
                        );
                    });
                });
                ui.add_space(4.0);

                // Minecraft inventories use nine columns, but the full 22-disc
                // catalogue fits in two clean rows at the default window size
                // when the panel is wide enough. Narrower windows fall back to
                // the familiar 9 / 6 / 5 / 4 column inventory widths.
                let cols = if inner_w >= 1000.0 {
                    11
                } else if inner_w >= 760.0 {
                    9
                } else if inner_w >= 560.0 {
                    6
                } else if inner_w >= 460.0 {
                    5
                } else {
                    4
                };
                let gap = 8.0;
                let card_w = ((inner_w - gap * (cols as f32 - 1.0)) / cols as f32)
                    .floor()
                    .max(80.0);
                let disc_size = (card_w * 0.46).clamp(38.0, 64.0);
                // Two title lines are reserved so long names such as
                // "Creator (Music Box)" never run into a neighbouring card.
                let card_h = disc_size + 70.0;
                ui.spacing_mut().item_spacing = Vec2::new(gap, gap);

                for row in DISCS.chunks(cols) {
                    ui.horizontal(|ui| {
                        for disc in row {
                            let index = DISCS
                                .iter()
                                .position(|d| d.id == disc.id)
                                .unwrap_or_default();
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(card_w, card_h),
                                Sense::click_and_drag(),
                            );
                            let hovered = response.hovered();
                            let selected = self.selected == index;
                            let found = self
                                .library
                                .as_ref()
                                .map(|lib| lib.has_disc(disc))
                                .unwrap_or(false);
                            let painter = ui.painter();
                            let bg = if selected {
                                Color32::from_rgb(49, 43, 29)
                            } else if hovered {
                                Color32::from_rgb(43, 47, 55)
                            } else {
                                PANEL_DARK
                            };
                            painter.rect_filled(rect, 0.0, bg);
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(
                                    if selected { 2.0_f32 } else { 1.0_f32 },
                                    if selected { GOLD } else { OUTLINE },
                                ),
                            );
                            style::paint_bevel(painter, rect.shrink(1.0), 1.0);

                            let slot = Rect::from_center_size(
                                Pos2::new(rect.center().x, rect.top() + 8.0 + disc_size * 0.5),
                                Vec2::splat(disc_size + 10.0),
                            );
                            painter.rect_filled(slot, 0.0, SLOT);
                            painter.rect_stroke(
                                slot,
                                0.0,
                                Stroke::new(1.0_f32, Color32::from_rgb(40, 43, 50)),
                            );
                            let disc_rect =
                                Rect::from_center_size(slot.center(), Vec2::splat(disc_size));
                            let tint = if found || self.library.is_none() {
                                Color32::WHITE
                            } else {
                                Color32::from_gray(82)
                            };
                            painter.image(
                                self.textures.discs[index].id(),
                                disc_rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                tint,
                            );

                            let dot = Rect::from_min_size(
                                Pos2::new(rect.right() - 10.0, rect.top() + 7.0),
                                Vec2::splat(4.0),
                            );
                            painter.rect_filled(
                                dot,
                                0.0,
                                if found {
                                    GREEN
                                } else if self.library.is_some() {
                                    RED
                                } else {
                                    MUTED_DARK
                                },
                            );

                            let title_y = slot.bottom() + 7.0;
                            let title_size = if disc.title.len() > 14 {
                                6.0
                            } else if disc.title.len() > 10 {
                                7.0
                            } else {
                                8.0
                            };
                            let title_font = FontId::proportional(title_size);
                            let wrap_width = (card_w - 8.0).max(40.0);
                            let title_color = if selected { GOLD } else { TEXT };
                            let title = painter.layout(
                                disc.title.to_owned(),
                                title_font.clone(),
                                title_color,
                                wrap_width,
                            );
                            let title_shadow = painter.layout(
                                disc.title.to_owned(),
                                title_font,
                                Color32::from_black_alpha(200),
                                wrap_width,
                            );
                            let title_x = rect.center().x - title.size().x / 2.0;
                            painter.galley(
                                Pos2::new(title_x + 1.0, title_y + 1.0),
                                title_shadow,
                                Color32::PLACEHOLDER,
                            );
                            let title_h = title.size().y;
                            painter.galley(
                                Pos2::new(title_x, title_y),
                                title,
                                Color32::PLACEHOLDER,
                            );
                            let artist_y = title_y + title_h + 3.0;
                            painter.text(
                                Pos2::new(rect.center().x, artist_y),
                                Align2::CENTER_TOP,
                                disc.artist,
                                FontId::proportional(6.0),
                                MUTED,
                            );
                            let status_y = artist_y + 10.0;
                            if self.library.is_some() && !found {
                                painter.text(
                                    Pos2::new(rect.center().x, status_y),
                                    Align2::CENTER_TOP,
                                    "MISSING",
                                    FontId::proportional(5.0),
                                    RED,
                                );
                            } else if selected {
                                painter.text(
                                    Pos2::new(rect.center().x, status_y),
                                    Align2::CENTER_TOP,
                                    "SELECTED",
                                    FontId::proportional(5.0),
                                    GOLD,
                                );
                            }

                            if hovered {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if response.clicked() {
                                clicked_select = Some(index);
                            }
                            if response.double_clicked() {
                                double_click = Some(index);
                            }
                            if response.drag_started() {
                                drag_start = Some(index);
                            }
                            if response.dragged() {
                                drag_update = ui.ctx().pointer_latest_pos();
                            }
                            if response.drag_stopped() {
                                drag_stop = Some((index, ui.ctx().pointer_latest_pos()));
                            }
                        }
                    });
                }
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            });
        });

        style::paint_bevel(ui.painter(), outer.response.rect.shrink(2.0), 2.0);
        ui.painter().rect_stroke(
            outer.response.rect.shrink(5.0),
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(50, 54, 62)),
        );

        if let Some(index) = clicked_select {
            self.selected = index;
        }
        if let Some(index) = double_click {
            self.begin_insert(index);
        }
        if let Some(index) = drag_start {
            self.drag_disc = Some(index);
            self.drag_pos = drag_update;
        }
        if let Some(pos) = drag_update {
            if self.drag_disc.is_some() {
                self.drag_pos = Some(pos);
            }
        }
        if let Some((index, pos)) = drag_stop {
            self.drag_disc = None;
            self.drag_pos = None;
            if let Some(pos) = pos {
                if self.jukebox_rect.expand(28.0).contains(pos) {
                    self.begin_insert(index);
                }
            }
        }
    }

    fn footer(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            style::pixel_label(
                ui,
                "DOUBLE-CLICK A DISC OR DRAG IT ONTO THE JUKEBOX",
                6.0,
                MUTED,
            );
            style::pixel_label(
                ui,
                "SPACE PLAY / EJECT   ARROW KEYS CHANGE DISC   REPEAT LOOPS THE TRACK",
                5.0,
                MUTED_DARK,
            );
        });
    }

    fn setup_modal(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let dim = ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("setup-dim")));
        dim.rect_filled(screen, 0.0, Color32::from_black_alpha(190));

        let mut choose = false;
        let mut use_default = false;
        let mut close = false;

        let frame = egui::Frame::none()
            .fill(PANEL)
            .stroke(Stroke::new(2.0_f32, OUTLINE))
            .inner_margin(Margin::same(20.0));

        let area = egui::Area::new(Id::new("setup-modal"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(620.0);
                    style::pixel_label(ui, "FIND YOUR MINECRAFT FOLDER", 12.0, GOLD);
                    ui.add_space(8.0);
                    ui.add(
                        Label::new(
                            RichText::new(
                                "Minecraft Jukebox plays the music discs already installed with Minecraft Java Edition. Your music stays on this Mac - nothing is copied or uploaded.",
                            )
                            .size(7.0)
                            .color(TEXT),
                        )
                        .wrap(),
                    );
                    ui.add_space(10.0);

                    let path_text = self
                        .config
                        .minecraft_dir
                        .clone()
                        .unwrap_or_else(|| {
                            assets::display_path(&assets::default_minecraft_dir())
                        });
                    let path_frame = egui::Frame::none()
                        .fill(PANEL_DEEP)
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(53, 57, 65)))
                        .inner_margin(Margin::symmetric(8.0, 7.0));
                    path_frame.show(ui, |ui| {
                        ui.set_width(580.0);
                        ui.add(
                            Label::new(
                                RichText::new(path_text).size(7.0).monospace().color(TEXT),
                            )
                            .wrap(),
                        );
                    });

                    if let Some(lib) = &self.library {
                        ui.add_space(6.0);
                        style::pixel_label(
                            ui,
                            &format!(
                                "FOUND {} OF {} DISC TRACKS",
                                lib.found_count(),
                                lib.total_count()
                            ),
                            7.0,
                            if lib.found_count() > 0 { GREEN } else { RED },
                        );
                        style::pixel_label(
                            ui,
                            &format!(
                                "ASSET INDEX: {} ({} INDEX FILES)",
                                lib.index_name, lib.index_count
                            ),
                            6.0,
                            MUTED,
                        );
                    }

                    if let Some(err) = &self.setup_error {
                        ui.add_space(6.0);
                        ui.add(
                            Label::new(RichText::new(err).size(7.0).color(RED)).wrap(),
                        );
                    }

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if style::mc_button(
                            ui,
                            "USE DEFAULT FOLDER",
                            Vec2::new(196.0, 34.0),
                            self.setup_default_available,
                        )
                        .clicked()
                        {
                            use_default = true;
                        }
                        ui.add_space(8.0);
                        if style::mc_button(
                            ui,
                            "CHOOSE FOLDER...",
                            Vec2::new(176.0, 34.0),
                            true,
                        )
                        .clicked()
                        {
                            choose = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if style::mc_button(ui, "NOT NOW", Vec2::new(100.0, 34.0), true)
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });

                    ui.add_space(12.0);
                    style::pixel_label(ui, "HOW TO FIND IT", 7.0, GOLD);
                    ui.add(
                        Label::new(
                            RichText::new(
                                "In the Minecraft Launcher, open Installations, hover a Java installation and choose Open Folder. Select the folder that contains 'assets'. The usual macOS path is ~/Library/Application Support/minecraft.",
                            )
                            .size(6.0)
                            .color(MUTED),
                        )
                        .wrap(),
                    );
                    ui.add_space(6.0);
                    style::pixel_label(
                        ui,
                        &format!("CONFIG: {}", self.config_path_display()),
                        5.0,
                        MUTED_DARK,
                    );
                });
            });

        let bevel_painter = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("setup-bevel"),
        ));
        style::paint_bevel(&bevel_painter, area.response.rect.shrink(2.0), 2.0);

        if choose {
            self.pick_folder();
        } else if use_default {
            self.use_default_folder();
        } else if close {
            self.setup_open = false;
        }
    }

    fn paint_drag(&self, ctx: &egui::Context) {
        let (Some(index), Some(pos)) = (self.drag_disc, self.drag_pos) else {
            return;
        };
        let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("drag-disc")));
        let rect = Rect::from_center_size(pos, Vec2::splat(76.0));
        painter.rect_filled(
            Rect::from_min_size(rect.min + Vec2::new(5.0, 6.0), rect.size()),
            0.0,
            Color32::from_black_alpha(110),
        );
        painter.image(
            self.textures.discs[index].id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if self.jukebox_rect.expand(28.0).contains(pos) {
            painter.rect_stroke(
                self.jukebox_rect.expand(8.0),
                0.0,
                Stroke::new(3.0_f32, GOLD),
            );
        }
    }

    fn paint_toast(&self, ctx: &egui::Context) {
        let Some((message, remaining)) = &self.toast else {
            return;
        };
        if *remaining <= 0.0 {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("toast")));
        let galley = painter.layout_no_wrap(message.clone(), FontId::proportional(7.0), TEXT);
        let size = galley.size() + Vec2::new(30.0, 18.0);
        let rect = Rect::from_center_size(
            Pos2::new(
                ctx.screen_rect().center().x,
                ctx.screen_rect().bottom() - 34.0,
            ),
            size,
        );
        painter.rect_filled(rect, 0.0, PANEL);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, GOLD));
        style::paint_bevel(&painter, rect.shrink(1.0), 1.0);
        painter.galley(
            rect.center() - galley.size() / 2.0,
            galley,
            Color32::PLACEHOLDER,
        );
    }
}

impl eframe::App for JukeboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.1);

        if let Some(path) = &self.screenshot_path {
            if let Some(image) = take_screenshot(ctx) {
                match save_screenshot(path, &image) {
                    Ok(()) => {
                        eprintln!("Screenshot saved to {}", path.display());
                        std::process::exit(0);
                    }
                    Err(err) => {
                        eprintln!("Could not save screenshot: {err}");
                        std::process::exit(2);
                    }
                }
            }
            if self.frame_count == 8 && !self.demo_started {
                self.demo_started = true;
                // Only show a disc in the screenshot when the selected
                // Minecraft installation really has it. Otherwise the shot
                // should show the empty jukebox and the friendly folder
                // banner instead of pretending music is available.
                let index = self.library.as_ref().and_then(|lib| {
                    DISCS
                        .iter()
                        .position(|disc| disc.id == "cat" && lib.has_disc(disc))
                        .or_else(|| DISCS.iter().position(|disc| lib.has_disc(disc)))
                });
                if let Some(index) = index {
                    self.selected = index;
                    self.begin_insert(index);
                }
            }
        }

        self.handle_input(ctx);
        self.advance(dt);
        self.render(ctx);

        if let Some(path) = &self.screenshot_path {
            if self.frame_count >= 120 {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(ViewportCommand::Screenshot);
            }
            if self.frame_count > 260 && self.screenshot_requested {
                eprintln!(
                    "Screenshot mode timed out while waiting for {}",
                    path.display()
                );
                std::process::exit(2);
            }
            ctx.request_repaint();
        } else if matches!(
            self.state,
            JukeboxState::Inserting | JukeboxState::Playing | JukeboxState::Ejecting
        ) || !self.notes.is_empty()
        {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        self.frame_count = self.frame_count.saturating_add(1);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Volume and repeat are saved on a short delay while the window is
        // open. Flush a pending change when the person closes the app.
        if self.config_dirty {
            self.save_config();
        }
    }
}

impl Drop for JukeboxApp {
    fn drop(&mut self) {
        // Flush a pending volume/repeat change if the window closes before the
        // debounce timer fires. Demo/headless runs never touch the real config.
        if self.config_dirty && !self.demo_mode {
            self.save_config();
        }
    }
}

fn take_screenshot(ctx: &egui::Context) -> Option<std::sync::Arc<egui::ColorImage>> {
    ctx.input(|input| {
        input.events.iter().find_map(|event| {
            if let egui::Event::Screenshot { image, .. } = event {
                Some(image.clone())
            } else {
                None
            }
        })
    })
}

fn save_screenshot(path: &Path, image: &egui::ColorImage) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let width = image.size[0] as u32;
    let height = image.size[1] as u32;
    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8)?;
    Ok(())
}

fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("MINECRAFT_JUKEBOX_CONFIG") {
        return PathBuf::from(path);
    }
    let base = dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("MinecraftJukebox").join("config.json")
}
