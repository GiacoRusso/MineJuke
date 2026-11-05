//! The Minecraft music disc catalogue.
//!
//! The list is deliberately kept in one place: the grid, the previous/next
//! controls and the asset lookup all read from here. The order follows the
//! complete 22-disc list supplied with the request.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Ring,
    Stripe,
    Cross,
    Diamond,
    Dots,
    Star,
    Wave,
    Split,
}

#[derive(Clone, Copy, Debug)]
pub struct Disc {
    /// Stable id used by the UI and config.
    pub id: &'static str,
    /// File stem inside `minecraft/sounds/records/`.
    pub asset: &'static str,
    pub title: &'static str,
    pub artist: &'static str,
    /// Main vinyl colour.
    pub color: (u8, u8, u8),
    /// Label / pattern colour.
    pub accent: (u8, u8, u8),
    pub pattern: Pattern,
}

pub const DISCS: [Disc; 22] = [
    Disc {
        id: "13",
        asset: "13",
        title: "13",
        artist: "C418",
        color: (196, 82, 45),
        accent: (255, 196, 94),
        pattern: Pattern::Ring,
    },
    Disc {
        id: "cat",
        asset: "cat",
        title: "Cat",
        artist: "C418",
        color: (122, 168, 64),
        accent: (230, 255, 150),
        pattern: Pattern::Stripe,
    },
    Disc {
        id: "blocks",
        asset: "blocks",
        title: "Blocks",
        artist: "C418",
        color: (58, 130, 78),
        accent: (128, 220, 120),
        pattern: Pattern::Dots,
    },
    Disc {
        id: "chirp",
        asset: "chirp",
        title: "Chirp",
        artist: "C418",
        color: (168, 54, 54),
        accent: (255, 140, 120),
        pattern: Pattern::Cross,
    },
    Disc {
        id: "far",
        asset: "far",
        title: "Far",
        artist: "C418",
        color: (98, 66, 160),
        accent: (200, 170, 255),
        pattern: Pattern::Star,
    },
    Disc {
        id: "mall",
        asset: "mall",
        title: "Mall",
        artist: "C418",
        color: (54, 102, 168),
        accent: (140, 200, 255),
        pattern: Pattern::Wave,
    },
    Disc {
        id: "mellohi",
        asset: "mellohi",
        title: "Mellohi",
        artist: "C418",
        color: (150, 62, 150),
        accent: (255, 160, 255),
        pattern: Pattern::Diamond,
    },
    Disc {
        id: "stal",
        asset: "stal",
        title: "Stal",
        artist: "C418",
        color: (56, 150, 160),
        accent: (150, 255, 245),
        pattern: Pattern::Ring,
    },
    Disc {
        id: "strad",
        asset: "strad",
        title: "Strad",
        artist: "C418",
        color: (188, 110, 140),
        accent: (255, 200, 220),
        pattern: Pattern::Split,
    },
    Disc {
        id: "ward",
        asset: "ward",
        title: "Ward",
        artist: "C418",
        color: (54, 92, 84),
        accent: (140, 200, 180),
        pattern: Pattern::Stripe,
    },
    Disc {
        id: "11",
        asset: "11",
        title: "11",
        artist: "C418",
        color: (92, 40, 44),
        accent: (200, 80, 80),
        pattern: Pattern::Cross,
    },
    Disc {
        id: "creator_music_box",
        asset: "creator_music_box",
        title: "Creator (Music Box)",
        artist: "Lena Raine",
        color: (186, 108, 70),
        accent: (255, 208, 120),
        pattern: Pattern::Ring,
    },
    Disc {
        id: "wait",
        asset: "wait",
        title: "Wait",
        artist: "C418",
        color: (72, 142, 190),
        accent: (170, 230, 255),
        pattern: Pattern::Wave,
    },
    Disc {
        id: "creator",
        asset: "creator",
        title: "Creator",
        artist: "Lena Raine",
        color: (48, 150, 160),
        accent: (120, 255, 240),
        pattern: Pattern::Ring,
    },
    Disc {
        id: "precipice",
        asset: "precipice",
        title: "Precipice",
        artist: "Aaron Cherof",
        color: (110, 140, 180),
        accent: (210, 235, 255),
        pattern: Pattern::Wave,
    },
    Disc {
        id: "otherside",
        asset: "otherside",
        title: "Otherside",
        artist: "Lena Raine",
        color: (96, 70, 160),
        accent: (180, 160, 255),
        pattern: Pattern::Diamond,
    },
    Disc {
        id: "relic",
        asset: "relic",
        title: "Relic",
        artist: "Aaron Cherof",
        color: (160, 112, 48),
        accent: (255, 210, 130),
        pattern: Pattern::Split,
    },
    Disc {
        id: "5",
        asset: "5",
        title: "5",
        artist: "Samuel Åberg",
        color: (40, 140, 104),
        accent: (120, 255, 200),
        pattern: Pattern::Star,
    },
    Disc {
        id: "pigstep",
        asset: "pigstep",
        title: "Pigstep",
        artist: "Lena Raine",
        color: (196, 144, 38),
        accent: (255, 230, 120),
        pattern: Pattern::Dots,
    },
    Disc {
        id: "tears",
        asset: "tears",
        title: "Tears",
        artist: "Amos Roddy",
        color: (222, 224, 230),
        accent: (150, 210, 255),
        pattern: Pattern::Wave,
    },
    Disc {
        id: "lava_chicken",
        asset: "lava_chicken",
        title: "Lava Chicken",
        artist: "Hyper Potions",
        color: (194, 58, 46),
        accent: (250, 238, 214),
        pattern: Pattern::Split,
    },
    Disc {
        id: "bounce",
        asset: "bounce",
        title: "Bounce",
        artist: "fingerspit",
        color: (76, 186, 96),
        accent: (220, 255, 140),
        pattern: Pattern::Star,
    },
];

pub fn format_time(seconds: f32) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0:00".to_string();
    }
    let total = seconds.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_is_complete_and_unique() {
        assert_eq!(DISCS.len(), 22);
        let expected = [
            "13",
            "cat",
            "blocks",
            "chirp",
            "far",
            "mall",
            "mellohi",
            "stal",
            "strad",
            "ward",
            "11",
            "creator_music_box",
            "wait",
            "creator",
            "precipice",
            "otherside",
            "relic",
            "5",
            "pigstep",
            "tears",
            "lava_chicken",
            "bounce",
        ];
        assert_eq!(DISCS.len(), expected.len());
        let mut ids = std::collections::HashSet::new();
        for (disc, expected_id) in DISCS.iter().zip(expected) {
            assert_eq!(disc.id, expected_id);
            assert!(ids.insert(disc.id), "duplicate disc id {}", disc.id);
            assert!(!disc.title.is_empty());
            assert!(!disc.artist.is_empty());
        }
    }

    #[test]
    fn every_disc_has_a_record_asset_name() {
        for disc in DISCS {
            assert!(
                disc.asset
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "unexpected asset name {}",
                disc.asset
            );
        }
    }

    #[test]
    fn time_formatting() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(9.6), "0:10");
        assert_eq!(format_time(125.2), "2:05");
    }
}
