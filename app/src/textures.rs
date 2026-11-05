//! Procedural pixel art, generated at startup.
//!
//! Every texture is uploaded with nearest-neighbour filtering and no mipmaps,
//! so it stays chunky at any window size. If the user drops a real PNG into
//! `assets/` we use it instead, otherwise the art below is original.

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use std::path::Path;

use crate::disc::{Disc, Pattern, DISCS};

pub struct Textures {
    pub background: TextureHandle,
    pub wood: TextureHandle,
    pub jukebox: TextureHandle,
    pub discs: Vec<TextureHandle>,
}

impl Textures {
    pub fn new(ctx: &egui::Context) -> Self {
        let background = load_optional_png("dirt.png").unwrap_or_else(make_dirt_texture);
        let wood = load_optional_png("wood.png").unwrap_or_else(make_wood_texture);
        let jukebox = load_optional_png("jukebox.png").unwrap_or_else(make_jukebox_texture);

        let repeat = TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Nearest,
            wrap_mode: egui::TextureWrapMode::Repeat,
            mipmap_mode: None,
        };
        let nearest = TextureOptions::NEAREST;

        let discs = DISCS
            .iter()
            .map(|disc| {
                ctx.load_texture(
                    format!("disc-{}", disc.id),
                    make_disc_texture(disc),
                    nearest,
                )
            })
            .collect();

        Self {
            background: ctx.load_texture("dirt-tile", background, repeat),
            wood: ctx.load_texture("wood-tile", wood, repeat),
            jukebox: ctx.load_texture("jukebox", jukebox, nearest),
            discs,
        }
    }
}

fn load_optional_png(name: &str) -> Option<ColorImage> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name);
    if !path.is_file() {
        return None;
    }
    let img = image::open(path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(69069));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn scale(c: (u8, u8, u8), f: f32) -> Color32 {
    rgb(
        (c.0 as f32 * f).clamp(0.0, 255.0) as u8,
        (c.1 as f32 * f).clamp(0.0, 255.0) as u8,
        (c.2 as f32 * f).clamp(0.0, 255.0) as u8,
    )
}

fn make_dirt_texture() -> ColorImage {
    const W: usize = 16;
    const H: usize = 16;
    let palette = [
        (61, 42, 28),
        (74, 51, 34),
        (84, 59, 39),
        (53, 36, 25),
        (68, 47, 31),
    ];
    let mut pixels = Vec::with_capacity(W * H);
    for y in 0..H {
        for x in 0..W {
            let n = hash(x as u32, y as u32, 7);
            let mut c = palette[(n as usize) % palette.len()];
            if n % 19 == 0 {
                c = (44, 30, 22);
            } else if n % 29 == 0 {
                c = (104, 77, 52);
            } else if n % 41 == 0 {
                c = (36, 25, 20);
            }
            // a few tiny stone/pebble flecks
            if n % 53 == 0 {
                c = (92, 88, 80);
            }
            pixels.push(rgb(c.0, c.1, c.2));
        }
    }
    ColorImage {
        size: [W, H],
        pixels,
    }
}

fn make_wood_texture() -> ColorImage {
    const W: usize = 32;
    const H: usize = 16;
    let mut pixels = Vec::with_capacity(W * H);
    for y in 0..H {
        let plank = y / 4;
        let base = 54 + (hash(0, plank as u32, 3) % 14) as u8;
        for x in 0..W {
            let n = hash(x as u32, y as u32, 11);
            let mut c = rgb(base + 34, base + 18, base);
            if y % 4 == 3 {
                c = rgb(
                    base.saturating_sub(8),
                    base.saturating_sub(12),
                    base.saturating_sub(14),
                );
            } else if n % 13 == 0 {
                c = rgb(base + 42, base + 24, base + 4);
            } else if n % 17 == 0 {
                c = rgb(base + 20, base + 10, base.saturating_sub(4));
            }
            pixels.push(c);
        }
    }
    ColorImage {
        size: [W, H],
        pixels,
    }
}

fn make_jukebox_texture() -> ColorImage {
    const W: usize = 64;
    const H: usize = 64;
    let mut pixels = vec![Color32::TRANSPARENT; W * H];
    let mut put = |x: usize, y: usize, c: Color32| {
        if x < W && y < H {
            pixels[y * W + x] = c;
        }
    };

    for y in 0..H {
        for x in 0..W {
            let edge = x < 2 || y < 2 || x >= W - 2 || y >= H - 2;
            let mut c = if edge {
                rgb(18, 12, 10)
            } else {
                let n = hash(x as u32, y as u32, 23);
                let plank = y / 8;
                let base = 82 + (hash(0, plank as u32, 5) % 12) as u8;
                let mut c = rgb(base + 36, base + 18, base.saturating_sub(2));
                if y % 8 == 0 {
                    c = rgb(base + 12, base + 4, base.saturating_sub(14));
                } else if n % 17 == 0 {
                    c = rgb(base + 48, base + 28, base + 6);
                } else if n % 29 == 0 {
                    c = rgb(base + 18, base + 6, base.saturating_sub(8));
                }
                c
            };
            // outer frame highlight / shadow
            if x == 2 || y == 2 {
                c = rgb(128, 84, 48);
            }
            if x == W - 3 || y == H - 3 {
                c = rgb(52, 32, 22);
            }
            put(x, y, c);
        }
    }

    // dark top slot
    for y in 7..18 {
        for x in 16..48 {
            let mut c = rgb(14, 11, 10);
            if y == 7 || y == 17 || x == 16 || x == 47 {
                c = rgb(5, 5, 6);
            } else if y == 8 {
                c = rgb(74, 52, 34);
            }
            put(x, y, c);
        }
    }
    for y in 10..15 {
        for x in 20..44 {
            put(x, y, rgb(4, 4, 5));
        }
    }

    // front inset panel
    for y in 27..55 {
        for x in 10..54 {
            let border = x == 10 || x == 53 || y == 27 || y == 54;
            let mut c = if border {
                rgb(48, 30, 20)
            } else {
                rgb(31, 21, 17)
            };
            if y == 28 && x > 10 && x < 53 {
                c = rgb(76, 51, 32);
            }
            put(x, y, c);
        }
    }

    // gold trim around the front panel
    for x in 12..52 {
        put(x, 29, rgb(150, 104, 44));
        put(x, 52, rgb(92, 60, 28));
    }
    for y in 29..53 {
        put(12, y, rgb(150, 104, 44));
        put(51, y, rgb(92, 60, 28));
    }

    // pixel music note on the front
    let note = rgb(255, 205, 82);
    let note_dark = rgb(184, 126, 36);
    for y in 36..46 {
        put(30, y, note);
        put(31, y, note);
    }
    for x in 30..39 {
        put(x, 36, note);
        put(x, 37, note_dark);
    }
    for x in 35..40 {
        put(x, 38, note);
        put(x, 39, note_dark);
    }
    for y in 43..49 {
        for x in 25..33 {
            let d = (x as i32 - 29).abs() + (y as i32 - 46).abs();
            if d < 4 {
                put(x, y, if d < 2 { note } else { note_dark });
            }
        }
    }

    // corner bolts
    for (bx, by) in [(5, 5), (58, 5), (5, 58), (58, 58)] {
        for y in by..by + 3 {
            for x in bx..bx + 3 {
                put(x, y, rgb(116, 112, 100));
            }
        }
        put(bx + 1, by + 1, rgb(46, 44, 40));
    }

    ColorImage {
        size: [W, H],
        pixels,
    }
}

fn make_disc_texture(disc: &Disc) -> ColorImage {
    const W: usize = 32;
    const H: usize = 32;
    let mut pixels = vec![Color32::TRANSPARENT; W * H];
    let cx = 15.5_f32;
    let cy = 15.5_f32;

    for y in 0..H {
        for x in 0..W {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d > 15.3 {
                continue;
            }
            let mut c = scale(disc.color, 1.0);
            if d > 14.2 {
                c = scale(disc.color, 0.55);
            } else if d > 12.8 {
                c = scale(disc.accent, 0.8);
            } else if d > 5.4 {
                // vinyl grooves
                let ring = d as i32;
                if ring % 2 == 0 {
                    c = scale(disc.color, 0.76);
                }
                if ring % 5 == 0 {
                    c = scale(disc.color, 1.18);
                }
                if pattern_pixel(disc.pattern, x, y) {
                    c = scale(disc.accent, 0.92);
                }
            } else if d > 1.6 {
                c = scale(disc.accent, 1.0);
                if d > 4.6 {
                    c = scale(disc.accent, 0.7);
                }
            } else {
                c = rgb(12, 10, 12);
            }

            // a chunky highlight on the upper-left
            if (x == 9 && (y == 10 || y == 11)) || (x == 10 && y == 9) {
                c = rgb(238, 238, 230);
            }
            pixels[y * W + x] = c;
        }
    }

    ColorImage {
        size: [W, H],
        pixels,
    }
}

fn pattern_pixel(pattern: Pattern, x: usize, y: usize) -> bool {
    let x = x as i32;
    let y = y as i32;
    match pattern {
        Pattern::Ring => {
            let dx = x - 16;
            let dy = y - 16;
            let d2 = dx * dx + dy * dy;
            (64..=81).contains(&d2)
        }
        Pattern::Stripe => (x + y) % 8 == 0 || (x - y).rem_euclid(8) == 0,
        Pattern::Cross => x == 16 || y == 16,
        Pattern::Diamond => {
            let d = (x - 16).abs() + (y - 16).abs();
            d == 8 || d == 9
        }
        Pattern::Dots => x % 6 == 0 && y % 6 == 0,
        Pattern::Star => {
            let dx = (x - 16).abs();
            let dy = (y - 16).abs();
            (dx <= 1 && (6..=11).contains(&dy)) || (dy <= 1 && (6..=11).contains(&dx))
        }
        Pattern::Wave => {
            let target = 16 + ((x as f32 * 0.75).sin() * 4.0) as i32;
            (y - target).abs() <= 1
        }
        Pattern::Split => (x > 16 && y > 16) || (x < 15 && y < 15),
    }
}
