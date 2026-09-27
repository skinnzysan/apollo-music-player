use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use crate::library::Track;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Off,
    Track,
    Queue,
}

impl LoopMode {
    pub fn next(&self) -> Self {
        match self {
            LoopMode::Off => LoopMode::Track,
            LoopMode::Track => LoopMode::Queue,
            LoopMode::Queue => LoopMode::Off,
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            LoopMode::Off => "Brak",
            LoopMode::Track => "Utwór",
            LoopMode::Queue => "Kolejka",
        }
    }
}

pub struct VisualizerSource<I> {
    inner: I,
    samples_buffer: Arc<Mutex<VecDeque<f32>>>,
    local_samples: Vec<f32>,
}

impl<I> VisualizerSource<I>
where
    I: Source<Item = f32>,
{
    pub fn new(inner: I, samples_buffer: Arc<Mutex<VecDeque<f32>>>) -> Self {
        Self {
            inner,
            samples_buffer,
            local_samples: Vec::with_capacity(128),
        }
    }
}

impl<I> Iterator for VisualizerSource<I>
where
    I: Source<Item = f32>,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        self.local_samples.push(sample);
        if self.local_samples.len() >= 128 {
            let mut q = self.samples_buffer.lock();
            for &s in &self.local_samples {
                q.push_back(s);
            }
            while q.len() > 4096 {
                q.pop_front();
            }
            self.local_samples.clear();
        }
        Some(sample)
    }
}

impl<I> Source for VisualizerSource<I>
where
    I: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

pub struct AudioEngine {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    pub samples_buffer: Arc<Mutex<VecDeque<f32>>>,
    pub current_track: Option<Track>,
    pub is_playing: bool,
    pub volume: f32,
    pub is_muted: bool,
    pub prev_volume: f32,
    pub loop_mode: LoopMode,
    pub shuffle: bool,
    seek_fallback_offset: Duration,
    track_start_pos: Duration,
}

impl AudioEngine {
    pub fn new() -> Self {
        let samples_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(4096)));
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("Ostrzeżenie: Nie udało się zainicjalizować wyjścia audio: {}", e);
                (None, None)
            }
        };

        let sink = stream_handle.as_ref().and_then(|h| Sink::try_new(h).ok());
        if let Some(ref s) = sink {
            s.set_volume(0.8);
        }

        Self {
            _stream: stream,
            stream_handle,
            sink,
            samples_buffer,
            current_track: None,
            is_playing: false,
            volume: 0.8,
            is_muted: false,
            prev_volume: 0.8,
            loop_mode: LoopMode::Off,
            shuffle: false,
            seek_fallback_offset: Duration::ZERO,
            track_start_pos: Duration::ZERO,
        }
    }

    pub fn play_track(&mut self, track: Track) -> Result<(), String> {
        self.stop();

        let file = File::open(&track.path)
            .map_err(|e| format!("Błąd otwarcia pliku: {}", e))?;
        let reader = BufReader::new(file);

        let decoder = Decoder::new(reader)
            .map_err(|e| format!("Błąd dekodowania audio: {}", e))?;

        let _sample_rate = decoder.sample_rate();
        let source = decoder.convert_samples::<f32>();
        let vis_source = VisualizerSource::new(source, self.samples_buffer.clone());

        // Create or reset sink
        if self.sink.is_none() {
            if let Some(ref handle) = self.stream_handle {
                self.sink = Sink::try_new(handle).ok();
            }
        }

        if let Some(ref sink) = self.sink {
            sink.set_volume(if self.is_muted { 0.0 } else { self.volume });
            sink.append(vis_source);
            sink.play();
            self.is_playing = true;
            self.current_track = Some(track);
            self.seek_fallback_offset = Duration::ZERO;
            self.track_start_pos = Duration::ZERO;
            Ok(())
        } else {
            Err("Brak dostępnego wyjścia audio".to_string())
        }
    }

    pub fn toggle_play_pause(&mut self) {
        if let Some(ref sink) = self.sink {
            if sink.is_paused() {
                sink.play();
                self.is_playing = true;
            } else {
                sink.pause();
                self.is_playing = false;
            }
        }
    }

    pub fn pause(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.pause();
            self.is_playing = false;
        }
    }

    pub fn resume(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.play();
            self.is_playing = true;
        }
    }

    pub fn stop(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
        // Recreate sink because rodio Sink::stop marks it as stopped
        if let Some(ref handle) = self.stream_handle {
            self.sink = Sink::try_new(handle).ok();
            if let Some(ref s) = self.sink {
                s.set_volume(if self.is_muted { 0.0 } else { self.volume });
            }
        }
        self.is_playing = false;
        self.samples_buffer.lock().clear();
    }

    pub fn seek_relative(&mut self, delta_secs: i64) {
        let current_pos = self.get_position();
        let total_dur = self
            .current_track
            .as_ref()
            .map(|t| Duration::from_secs(t.duration_secs))
            .unwrap_or(Duration::from_secs(3600));

        let current_secs = current_pos.as_secs_f64();
        let target_secs = (current_secs + delta_secs as f64).clamp(0.0, total_dur.as_secs_f64());
        let target_dur = Duration::from_secs_f64(target_secs);

        self.seek_to(target_dur);
    }

    pub fn seek_to(&mut self, target: Duration) {
        if let Some(ref sink) = self.sink {
            // Try native seek first
            if sink.try_seek(target).is_ok() {
                return;
            }
        }

        // If native seek failed, reload and fast-forward
        if let Some(track) = self.current_track.clone() {
            let was_playing = self.is_playing;
            if let Ok(file) = File::open(&track.path) {
                let reader = BufReader::new(file);
                if let Ok(mut decoder) = Decoder::new(reader) {
                    if decoder.try_seek(target).is_err() {
                        let sr = decoder.sample_rate();
                        let ch = decoder.channels();
                        let to_skip = (target.as_secs_f64() * sr as f64 * ch as f64) as usize;
                        if to_skip > 0 {
                            let _ = decoder.nth(to_skip - 1);
                        }
                    }
                    let source = decoder.convert_samples::<f32>();
                    let vis_source = VisualizerSource::new(source, self.samples_buffer.clone());

                    if let Some(ref handle) = self.stream_handle {
                        self.sink = Sink::try_new(handle).ok();
                        if let Some(ref sink) = self.sink {
                            sink.set_volume(if self.is_muted { 0.0 } else { self.volume });
                            sink.append(vis_source);
                            self.seek_fallback_offset = target;
                            if was_playing {
                                sink.play();
                                self.is_playing = true;
                            } else {
                                sink.pause();
                                self.is_playing = false;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_position(&self) -> Duration {
        if let Some(ref sink) = self.sink {
            sink.get_pos() + self.seek_fallback_offset
        } else {
            Duration::ZERO
        }
    }

    pub fn is_track_finished(&self) -> bool {
        if let Some(ref sink) = self.sink {
            self.is_playing && sink.empty()
        } else {
            false
        }
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        self.is_muted = false;
        if let Some(ref sink) = self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn change_volume(&mut self, delta: f32) {
        let new_vol = (self.volume + delta).clamp(0.0, 1.0);
        self.set_volume(new_vol);
    }

    pub fn toggle_mute(&mut self) {
        if self.is_muted {
            self.is_muted = false;
            self.volume = self.prev_volume;
            if let Some(ref sink) = self.sink {
                sink.set_volume(self.volume);
            }
        } else {
            self.is_muted = true;
            self.prev_volume = self.volume;
            if let Some(ref sink) = self.sink {
                sink.set_volume(0.0);
            }
        }
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
    }

    pub fn cycle_loop_mode(&mut self) {
        self.loop_mode = self.loop_mode.next();
    }
}
