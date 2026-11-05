//! Audio playback on a dedicated rodio sink.
//!
//! The UI never waits for audio: playback happens on cpal's audio thread and
//! the app only reads the sink position. If there is no audio device (or the
//! user asks for a screenshot/self-test), the same timer is driven in software
//! so the interface remains testable and never freezes.

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Player {
    /// Kept alive for as long as playback: dropping it closes the audio device.
    #[allow(dead_code)]
    stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    /// The file currently loaded, so seeking can rebuild the sink from it.
    path: Option<PathBuf>,
    /// Samples before this point were skipped when rebuilding the sink, so
    /// `elapsed` has to add it to the sink's own position.
    seek_offset: Duration,
    volume: f32,
    duration: Option<Duration>,
    started: bool,
    simulated: bool,
    sim_elapsed: Duration,
    sim_duration: Duration,
    pub audio_error: Option<String>,
}

/// Accurate duration for an Ogg Vorbis file.
///
/// The decoder's `total_duration` is unreliable for Ogg (lewton reports none,
/// and rodio 0.20's symphonia path miscalculates the fractional part), so read
/// the last granule position from the Ogg pages and divide by the sample rate.
fn probe_duration(path: &Path, decoder: &Decoder<BufReader<File>>) -> Option<Duration> {
    let sample_rate = decoder.sample_rate();
    ogg_granule_duration(path, sample_rate).or_else(|| decoder.total_duration())
}

fn ogg_granule_duration(path: &Path, sample_rate: u32) -> Option<Duration> {
    if sample_rate == 0 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let mut offset = 0usize;
    let mut last_granule: Option<u64> = None;
    while offset + 27 <= bytes.len() {
        if &bytes[offset..offset + 4] != b"OggS" {
            break;
        }
        let granule = i64::from_le_bytes(bytes[offset + 6..offset + 14].try_into().ok()?);
        let segments = bytes[offset + 26] as usize;
        let table_end = offset + 27 + segments;
        if table_end > bytes.len() {
            break;
        }
        let data_len: usize = bytes[offset + 27..table_end]
            .iter()
            .map(|count| *count as usize)
            .sum();
        let next = table_end + data_len;
        if next > bytes.len() {
            break;
        }
        if granule >= 0 {
            last_granule = Some(granule as u64);
        }
        offset = next;
    }
    let samples = last_granule?;
    Some(Duration::from_secs_f64(samples as f64 / sample_rate as f64))
}

impl Player {
    pub fn new() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                stream: Some(stream),
                handle: Some(handle),
                sink: None,
                path: None,
                seek_offset: Duration::ZERO,
                volume: 0.7,
                duration: None,
                started: false,
                simulated: false,
                sim_elapsed: Duration::ZERO,
                sim_duration: Duration::ZERO,
                audio_error: None,
            },
            Err(err) => Self {
                stream: None,
                handle: None,
                sink: None,
                path: None,
                seek_offset: Duration::ZERO,
                volume: 0.7,
                duration: None,
                started: false,
                simulated: true,
                sim_elapsed: Duration::ZERO,
                sim_duration: Duration::ZERO,
                audio_error: Some(format!("No audio output device: {err}")),
            },
        }
    }

    /// Used by the screenshot and self-test modes: no sound, real state machine.
    pub fn simulated() -> Self {
        Self {
            stream: None,
            handle: None,
            sink: None,
            path: None,
            seek_offset: Duration::ZERO,
            volume: 0.7,
            duration: None,
            started: false,
            simulated: true,
            sim_elapsed: Duration::ZERO,
            sim_duration: Duration::ZERO,
            audio_error: None,
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
    }

    /// Start a real Ogg Vorbis file. Falls back to a silent software timer if
    /// an output device cannot be opened, so the UI still behaves correctly.
    pub fn play(&mut self, path: &Path) -> Result<Duration> {
        self.stop();

        let file =
            File::open(path).with_context(|| format!("Could not open {}", path.display()))?;
        let decoder = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Could not decode Ogg Vorbis {}", path.display()))?;
        let duration = probe_duration(path, &decoder).unwrap_or_else(|| Duration::from_secs(180));

        let mut sink_ok = false;
        if let Some(handle) = &self.handle {
            match Sink::try_new(handle) {
                Ok(sink) => {
                    sink.set_volume(self.volume);
                    sink.append(decoder);
                    self.sink = Some(sink);
                    sink_ok = true;
                }
                Err(err) => {
                    self.audio_error = Some(format!("Could not start audio: {err}"));
                }
            }
        }

        self.duration = Some(duration);
        self.path = Some(path.to_path_buf());
        self.seek_offset = Duration::ZERO;
        self.started = true;
        self.simulated = !sink_ok;
        self.sim_elapsed = Duration::ZERO;
        self.sim_duration = duration;
        Ok(duration)
    }

    pub fn play_simulated(&mut self, duration: Duration) {
        self.stop();
        self.duration = Some(duration);
        self.seek_offset = Duration::ZERO;
        self.started = true;
        self.simulated = true;
        self.sim_elapsed = Duration::ZERO;
        self.sim_duration = duration;
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.started = false;
        self.simulated = false;
        self.sim_elapsed = Duration::ZERO;
        self.duration = None;
        self.path = None;
        self.seek_offset = Duration::ZERO;
    }

    pub fn advance(&mut self, dt: Duration) {
        if self.started && self.simulated && !self.is_finished() {
            self.sim_elapsed = (self.sim_elapsed + dt).min(self.sim_duration);
        }
    }

    pub fn elapsed(&self) -> Duration {
        if self.started && !self.simulated {
            if let Some(sink) = &self.sink {
                return self.seek_offset + sink.get_pos();
            }
        }
        self.sim_elapsed
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn is_finished(&self) -> bool {
        if !self.started {
            return false;
        }
        if self.simulated {
            self.sim_elapsed >= self.sim_duration && self.sim_duration > Duration::ZERO
        } else if let Some(sink) = &self.sink {
            // A sink can be momentarily empty while the queue starts. The app
            // only polls this while Playing, after the insert animation.
            sink.empty()
        } else {
            true
        }
    }

    pub fn seek(&mut self, position: Duration) {
        let duration = self.duration.unwrap_or(position);
        let clamped = position.min(duration);
        if self.simulated {
            self.sim_elapsed = clamped;
            return;
        }

        // rodio's `Sink::try_seek` hands the seek to symphonia's Ogg demuxer,
        // which panics on Minecraft's records because the physical end of the
        // stream was never scanned (`phys_byte_range_end.unwrap()` on None).
        // Instead, rebuild the sink and decode past the samples up to the
        // requested position; this never seeks the demuxer and cannot abort
        // the audio thread.
        let Some(path) = self.path.clone() else {
            self.simulated = true;
            self.sim_elapsed = clamped;
            return;
        };
        let Some(handle) = self.handle.clone() else {
            self.simulated = true;
            self.sim_elapsed = clamped;
            return;
        };
        let Ok(file) = File::open(&path) else {
            return;
        };
        let Ok(decoder) = Decoder::new(BufReader::new(file)) else {
            return;
        };

        if let Some(old) = self.sink.take() {
            old.stop();
        }
        match Sink::try_new(&handle) {
            Ok(sink) => {
                sink.set_volume(self.volume);
                if clamped.is_zero() {
                    sink.append(decoder);
                } else {
                    sink.append(decoder.skip_duration(clamped));
                }
                self.sink = Some(sink);
                self.seek_offset = clamped;
                self.started = true;
                self.simulated = false;
                self.sim_elapsed = clamped;
            }
            Err(err) => {
                self.audio_error = Some(format!("Could not restart audio: {err}"));
                self.seek_offset = clamped;
                self.started = true;
                self.simulated = true;
                self.sim_elapsed = clamped;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{default_minecraft_dir, AssetLibrary};

    #[test]
    fn decodes_a_real_record_when_available() {
        let dir = default_minecraft_dir();
        let Ok(library) = AssetLibrary::load(&dir) else {
            return;
        };
        let Some(path) = library.records.values().next() else {
            return;
        };
        let file = File::open(path).expect("record should open");
        let decoder = Decoder::new(BufReader::new(file)).expect("record should decode");
        let duration = probe_duration(path, &decoder).expect("record should report a duration");
        assert!(duration > Duration::from_secs(10));
        assert!(duration < Duration::from_secs(600));
        println!("decoded {} ({duration:?})", path.display());
    }
}
