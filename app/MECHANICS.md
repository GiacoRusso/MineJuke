# Minecraft Jukebox - mechanics sheet

This is a player, not a combat game, so the "mechanics" are the timing,
navigation, layout and playback rules that make the jukebox feel like the
Minecraft block. There is no reference build in this workspace, so every value
that the request did not specify is marked **estimated**. The request values are
marked **request**.

## Discs and inventory

| Setting | Value | Source |
|---|---|---|
| Music discs in the catalogue | 22 | request follow-up |
| Discs, in display order: 13, cat, blocks, chirp, far, mall, mellohi, stal, strad, ward, 11, Creator (Music Box), wait, Creator, Precipice, Otherside, Relic, 5, Pigstep, Tears, Lava Chicken, Bounce | exact list | request follow-up |
| Grid columns at the default window width | 11 (two rows of 22 discs) | estimated |
| Grid columns at medium width | 9 (three rows) | estimated |
| Grid columns at narrower widths | 6 / 5 / 4 | estimated |
| Disc card width | computed from available width, 80 px minimum | estimated |
| Disc artwork size in a card | 38-64 px, nearest-neighbour | estimated |
| Selected disc highlight | 2 px gold border + gold title | estimated |
| Missing track marker | 4 px red square + "MISSING" | estimated |

The four discs added in the follow-up use the composers shown on the Minecraft
Wiki's Music Disc page: Creator (Music Box) - Lena Raine, Tears - Amos Roddy,
Lava Chicken - Hyper Potions, Bounce - fingerspit. Their colours are original
procedural pixel-art palettes, not the game's textures.

## Insert / eject animation

| Setting | Value | Source |
|---|---|---|
| Insert animation duration | 0.50 s | estimated |
| Insert motion | disc rises from 45% below the jukebox centre, scales 0.18 to 1.0, fades in and ends exactly in the playing pose | estimated |
| Insert easing | smoothstep (ease in-out) | estimated |
| Eject animation duration | 0.45 s | estimated |
| Eject motion | disc starts from the full playing pose, rises 32% of jukebox height, scales 1.0 to 0.40, fades out | estimated |
| Jukebox idle bob while playing | 2 px amplitude, 4 rad/s | estimated |
| Disc spin while playing | 1.75 rad/s | estimated |
| Disc spin while inserting | 1.35 rad/s | estimated |
| Jukebox art size | 250 px, clamped to 42% of panel width | estimated |
| Jukebox slot | top 11-28% of the block, centred | estimated |
| Insert sparkles | 7 squares orbiting the slot for the insert duration | estimated |

## Music notes

| Setting | Value | Source |
|---|---|---|
| Note spawn interval | 0.28 s | estimated |
| Note lifetime | 2.6 s (0.8-1.15x random) | estimated |
| Note rise speed | 48 px/s (0.85-1.3x random) | estimated |
| Note horizontal sway | 10 px/s amplitude (5-14 random) | estimated |
| Maximum live notes | 40 | estimated |
| Note glyph variants | eighth note, beamed pair, sparkle, quarter note | estimated |
| Note colours | gold, cyan, magenta, green, orange, purple, white plus the disc accent | estimated |
| Notes stop on eject or track end | yes | request |

## Playback and controls

| Setting | Value | Source |
|---|---|---|
| Default volume | 70% | estimated |
| Volume range | 0-100% | request |
| Now Playing line | `NOW PLAYING: Artist - Title` | request |
| Progress seek | click/drag on the bar | estimated |
| Duration fallback when Ogg probe fails | 180 s | estimated |
| Audio decoder | Ogg Vorbis via rodio + symphonia | request |
| Repeat default | off | estimated |
| Space key | play / eject | request |
| Left / right arrows | previous / next disc (wraps) | request |
| Up / down arrows | previous / next disc (wraps) | estimated |
| Enter | insert selected disc | estimated |
| Escape | close the folder dialog | estimated |
| Double-click a disc | insert it | request |
| Drag a disc onto the jukebox | insert it | request |
| Auto-eject at end of track | yes, unless repeat is on | request |
| Repeat behaviour | restart the same disc without ejecting | request |
| Track end detection | rodio sink empty; software timer in demo mode | estimated |
| Progress update | every frame (about 60 Hz) | estimated |
| Progress bar click/drag | seeks within the track | estimated |
| Time display | m:ss for elapsed and duration | estimated |
| Duration source | last Ogg granule position divided by the decoder sample rate | rodio API |
| Audio output | dedicated rodio `Sink` on the default output device | request |
| Audio fallback | silent software timer if no output device opens | estimated |

## Folder and assets

| Setting | Value | Source |
|---|---|---|
| Default Minecraft folder | `~/Library/Application Support/minecraft` | request |
| Folder picker | native macOS picker via `rfd` | request |
| Accepted folder | `.minecraft` root or its `assets` child | estimated |
| Asset index choice | newest `assets/indexes/*.json` by numeric version, then modified time | request |
| Sound key | `minecraft/sounds/records/<disc>.ogg` | request |
| Object path | `assets/objects/<first two hash chars>/<hash>` | request |
| Config location | `~/Library/Application Support/MinecraftJukebox/config.json` (or the platform config dir) | estimated |
| Config save delay after volume/repeat change | 0.6 s | estimated |
| Toast duration | 3.8 s | estimated |
| Missing files | friendly banner and folder dialog; no crash | request |

## Window and presentation

| Setting | Value | Source |
|---|---|---|
| Default window | 1180 x 900 | estimated |
| Minimum window | 900 x 680 | estimated |
| Window resizable | yes | request |
| Theme | dark | request |
| Pixel filtering | nearest-neighbour for every texture | request |
| Background tile | 64 px dirt texture, nearest-neighbour, repeat wrap | estimated |
| Panel style | 2 px dark border, 2 px bevel, 1 px inner line | estimated |
| Button hover | lighter stone fill + 1 px white inner outline | estimated |
| Button press | inverted bevel and 1 px text offset | estimated |
| Pixel font | Press Start 2P, embedded with `include_bytes!` | request |
| Screenshot warm-up | 120 frames | request/tool |
| Demo track length in screenshot/self-test | 185 s | estimated |

## Core loop measured by `--self-test`

1. **move** - an arrow-key event changes the selected disc.
2. **act** - a space-key event inserts a disc; after 2.5 s the state is
   `Playing` and the playback timer is over 1.5 s.
3. **hud** - a rendered egui frame contains "NOW PLAYING", the artist/title
   line, and the repeat/volume controls.
