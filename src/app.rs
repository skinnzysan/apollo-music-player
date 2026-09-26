use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam_channel::Receiver;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::audio::{AudioEngine, LoopMode};
use crate::library::{
    is_supported_audio_file, load_cached_library, start_background_scanner,
    AlbumInfo, GenreInfo, LibraryTab, ScannerMessage, Track,
};
use crate::visualizer::AudioVisualizer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    Library,
    Visualizer,
    Controls,
}

#[derive(Debug, Clone)]
pub enum ArtistTreeRow {
    ArtistHeader {
        artist: String,
        expanded: bool,
        album_count: usize,
        track_count: usize,
    },
    AlbumHeader {
        artist: String,
        album: String,
        year: Option<u32>,
        expanded: bool,
        track_count: usize,
    },
    TrackItem {
        track: Track,
        is_current: bool,
    },
}

#[derive(Debug, Clone)]
pub struct ExplorerItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_audio: bool,
}

pub struct App {
    pub audio: AudioEngine,
    pub visualizer: AudioVisualizer,
    pub tracks: Vec<Track>,
    pub active_tab: LibraryTab,
    pub focus_panel: FocusPanel,
    pub selected_index: usize,

    // Artists view state
    pub expanded_artists: HashSet<String>,
    pub expanded_albums: HashSet<(String, String)>,

    // Explorer view state
    pub explorer_dir: PathBuf,
    pub explorer_items: Vec<ExplorerItem>,

    // Queue state
    pub queue: Vec<Track>,
    pub current_queue_index: usize,
    pub shuffle_history: Vec<usize>,

    // Search state
    pub is_searching: bool,
    pub search_query: String,

    // Status / Notification
    pub status_message: Option<(String, Instant)>,

    // Background scanner
    pub scanner_rx: Option<Receiver<ScannerMessage>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub is_scanning: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(music_paths: Vec<PathBuf>) -> Self {
        let cached = load_cached_library().unwrap_or_default();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let scanner_rx = Some(start_background_scanner(
            music_paths.clone(),
            cached.clone(),
            cancel_flag.clone(),
        ));

        let home_music = dirs::audio_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Music")))
            .unwrap_or_else(|| PathBuf::from("."));

        let mut app = Self {
            audio: AudioEngine::new(),
            visualizer: AudioVisualizer::new(48000),
            tracks: cached,
            active_tab: LibraryTab::Artists,
            focus_panel: FocusPanel::Library,
            selected_index: 0,
            expanded_artists: HashSet::new(),
            expanded_albums: HashSet::new(),
            explorer_dir: home_music.clone(),
            explorer_items: Vec::new(),
            queue: Vec::new(),
            current_queue_index: 0,
            shuffle_history: Vec::new(),
            is_searching: false,
            search_query: String::new(),
            status_message: Some((
                "Witaj w Apollo! [Spacja] Odtwarzaj | [/] Szukaj".to_string(),
                Instant::now(),
            )),
            scanner_rx,
            cancel_flag,
            is_scanning: true,
            should_quit: false,
        };

        app.refresh_explorer();
        app.auto_expand_first();
        app
    }

    pub fn auto_expand_first(&mut self) {
        if let Some(first_track) = self.tracks.first() {
            self.expanded_artists.insert(first_track.artist.clone());
            self.expanded_albums.insert((first_track.artist.clone(), first_track.album.clone()));
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn get_status(&self) -> Option<&str> {
        if let Some((ref msg, time)) = self.status_message {
            if time.elapsed() < Duration::from_secs(5) {
                return Some(msg.as_str());
            }
        }
        None
    }

    pub fn check_scanner_messages(&mut self) {
        if let Some(ref rx) = self.scanner_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScannerMessage::Batch(batch) => {
                        for track in batch {
                            if let Some(idx) = self.tracks.iter().position(|t| t.path == track.path) {
                                self.tracks[idx] = track;
                            } else {
                                self.tracks.push(track);
                            }
                        }
                    }
                    ScannerMessage::Finished(count) => {
                        self.is_scanning = false;
                        self.set_status(format!("Zakończono skanowanie. Liczba utworów: {}", count));
                        self.auto_expand_first();
                        break;
                    }
                }
            }
        }
    }

    pub fn check_playback_progress(&mut self) {
        if self.audio.is_track_finished() {
            self.on_track_ended();
        }
    }

    pub fn on_track_ended(&mut self) {
        match self.audio.loop_mode {
            LoopMode::Track => {
                if let Some(ref current) = self.audio.current_track.clone() {
                    let _ = self.audio.play_track(current.clone());
                }
            }
            LoopMode::Queue => {
                self.next_track(true);
            }
            LoopMode::Off => {
                self.next_track(false);
            }
        }
    }

    pub fn play_track_at_queue_index(&mut self, index: usize) {
        if index >= self.queue.len() {
            return;
        }
        self.current_queue_index = index;
        let track = self.queue[index].clone();
        if let Some(rate) = track.sample_rate {
            self.visualizer.set_sample_rate(rate);
        }
        if let Err(e) = self.audio.play_track(track.clone()) {
            self.set_status(format!("Błąd odtwarzania: {}", e));
        } else {
            self.set_status(format!("Odtwarzanie: {} - {}", track.artist, track.display_title()));
        }
    }

    pub fn next_track(&mut self, wrap_around: bool) {
        if self.queue.is_empty() {
            return;
        }
        if self.audio.shuffle {
            let mut rng = thread_rng();
            let next_idx = (0..self.queue.len())
                .filter(|&i| i != self.current_queue_index || self.queue.len() == 1)
                .collect::<Vec<_>>()
                .choose(&mut rng)
                .copied()
                .unwrap_or(0);
            self.play_track_at_queue_index(next_idx);
        } else if self.current_queue_index + 1 < self.queue.len() {
            self.play_track_at_queue_index(self.current_queue_index + 1);
        } else if wrap_around {
            self.play_track_at_queue_index(0);
        } else {
            self.audio.stop();
            self.set_status("Koniec kolejki odtwarzania");
        }
    }

    pub fn previous_track(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        if self.audio.get_position().as_secs() > 3 {
            self.audio.seek_to(Duration::ZERO);
            return;
        }
        if self.current_queue_index > 0 {
            self.play_track_at_queue_index(self.current_queue_index - 1);
        } else {
            self.play_track_at_queue_index(self.queue.len().saturating_sub(1));
        }
    }

    pub fn get_filtered_tracks(&self) -> Vec<Track> {
        self.tracks
            .iter()
            .filter(|t| t.matches_filter(&self.search_query))
            .cloned()
            .collect()
    }

    pub fn get_artist_tree_rows(&self) -> Vec<ArtistTreeRow> {
        let filtered = self.get_filtered_tracks();
        let current_path = self.audio.current_track.as_ref().map(|t| &t.path);

        // Group by artist -> album -> tracks
        let mut map: BTreeMap<String, BTreeMap<String, Vec<Track>>> = BTreeMap::new();
        for track in filtered {
            map.entry(track.artist.clone())
                .or_default()
                .entry(track.album.clone())
                .or_default()
                .push(track);
        }

        let mut rows = Vec::new();
        for (artist, albums) in map {
            let total_tracks: usize = albums.values().map(|v| v.len()).sum();
            let is_artist_expanded = self.expanded_artists.contains(&artist);

            rows.push(ArtistTreeRow::ArtistHeader {
                artist: artist.clone(),
                expanded: is_artist_expanded,
                album_count: albums.len(),
                track_count: total_tracks,
            });

            if is_artist_expanded {
                for (album, mut tracks) in albums {
                    tracks.sort_by_key(|t| (t.track_number.unwrap_or(0), t.title.clone()));
                    let is_album_expanded =
                        self.expanded_albums.contains(&(artist.clone(), album.clone()));
                    let year = tracks.iter().find_map(|t| t.year);

                    rows.push(ArtistTreeRow::AlbumHeader {
                        artist: artist.clone(),
                        album: album.clone(),
                        year,
                        expanded: is_album_expanded,
                        track_count: tracks.len(),
                    });

                    if is_album_expanded {
                        for track in tracks {
                            let is_current = current_path == Some(&track.path);
                            rows.push(ArtistTreeRow::TrackItem { track, is_current });
                        }
                    }
                }
            }
        }

        rows
    }

    pub fn get_album_list(&self) -> Vec<AlbumInfo> {
        let filtered = self.get_filtered_tracks();
        let mut map: BTreeMap<(String, String), Vec<Track>> = BTreeMap::new();
        for track in filtered {
            map.entry((track.artist.clone(), track.album.clone()))
                .or_default()
                .push(track);
        }

        let mut albums = Vec::new();
        for ((artist, title), mut tracks) in map {
            tracks.sort_by_key(|t| (t.track_number.unwrap_or(0), t.title.clone()));
            let year = tracks.iter().find_map(|t| t.year);
            let total_duration_secs = tracks.iter().map(|t| t.duration_secs).sum();
            let count = tracks.len();
            albums.push(AlbumInfo {
                title,
                artist,
                year,
                track_count: count,
                total_duration_secs,
                tracks,
            });
        }
        albums
    }

    pub fn get_genre_list(&self) -> Vec<GenreInfo> {
        let filtered = self.get_filtered_tracks();
        let mut map: BTreeMap<String, Vec<Track>> = BTreeMap::new();
        for track in filtered {
            let g = track
                .genre
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or("Nieokreślony")
                .to_string();
            map.entry(g).or_default().push(track);
        }

        let mut genres = Vec::new();
        for (name, tracks) in map {
            let count = tracks.len();
            genres.push(GenreInfo {
                name,
                track_count: count,
                tracks,
            });
        }
        genres
    }

    pub fn refresh_explorer(&mut self) {
        self.explorer_items.clear();
        if let Ok(entries) = std::fs::read_dir(&self.explorer_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = path.is_dir();
                let is_audio = is_supported_audio_file(&path);
                if is_dir {
                    dirs.push(ExplorerItem {
                        path,
                        name,
                        is_dir: true,
                        is_audio: false,
                    });
                } else if is_audio {
                    files.push(ExplorerItem {
                        path,
                        name,
                        is_dir: false,
                        is_audio: true,
                    });
                }
            }

            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            if let Some(parent) = self.explorer_dir.parent() {
                self.explorer_items.push(ExplorerItem {
                    path: parent.to_path_buf(),
                    name: ".. [Katalog wyżej]".to_string(),
                    is_dir: true,
                    is_audio: false,
                });
            }

            self.explorer_items.extend(dirs);
            self.explorer_items.extend(files);
        }
    }

    pub fn handle_enter_key(&mut self) {
        match self.active_tab {
            LibraryTab::Artists => {
                let rows = self.get_artist_tree_rows();
                if let Some(row) = rows.get(self.selected_index) {
                    match row {
                        ArtistTreeRow::ArtistHeader { artist, .. } => {
                            if self.expanded_artists.contains(artist) {
                                self.expanded_artists.remove(artist);
                            } else {
                                self.expanded_artists.insert(artist.clone());
                            }
                        }
                        ArtistTreeRow::AlbumHeader { artist, album, .. } => {
                            let key = (artist.clone(), album.clone());
                            if self.expanded_albums.contains(&key) {
                                self.expanded_albums.remove(&key);
                            } else {
                                self.expanded_albums.insert(key);
                            }
                        }
                        ArtistTreeRow::TrackItem { track, .. } => {
                            // Collect tracks from the same album as queue
                            let album_tracks: Vec<Track> = self
                                .tracks
                                .iter()
                                .filter(|t| t.artist == track.artist && t.album == track.album)
                                .cloned()
                                .collect();

                            let queue = if !album_tracks.is_empty() {
                                album_tracks
                            } else {
                                vec![track.clone()]
                            };

                            let start_idx = queue
                                .iter()
                                .position(|t| t.path == track.path)
                                .unwrap_or(0);

                            self.queue = queue;
                            self.play_track_at_queue_index(start_idx);
                        }
                    }
                }
            }
            LibraryTab::Albums => {
                let albums = self.get_album_list();
                if let Some(album) = albums.get(self.selected_index) {
                    if !album.tracks.is_empty() {
                        self.queue = album.tracks.clone();
                        self.play_track_at_queue_index(0);
                    }
                }
            }
            LibraryTab::Tracks => {
                let tracks = self.get_filtered_tracks();
                if tracks.get(self.selected_index).is_some() {
                    self.queue = tracks;
                    self.play_track_at_queue_index(self.selected_index);
                }
            }
            LibraryTab::Genres => {
                let genres = self.get_genre_list();
                if let Some(genre) = genres.get(self.selected_index) {
                    if !genre.tracks.is_empty() {
                        self.queue = genre.tracks.clone();
                        self.play_track_at_queue_index(0);
                    }
                }
            }
            LibraryTab::Explorer => {
                if let Some(item) = self.explorer_items.get(self.selected_index).cloned() {
                    if item.is_dir {
                        self.explorer_dir = item.path;
                        self.selected_index = 0;
                        self.refresh_explorer();
                    } else if item.is_audio {
                        if let Some(track) = crate::library::parse_track_metadata(&item.path) {
                            self.queue = vec![track];
                            self.play_track_at_queue_index(0);
                        }
                    }
                }
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        let max_len = match self.active_tab {
            LibraryTab::Artists => self.get_artist_tree_rows().len(),
            LibraryTab::Albums => self.get_album_list().len(),
            LibraryTab::Tracks => self.get_filtered_tracks().len(),
            LibraryTab::Genres => self.get_genre_list().len(),
            LibraryTab::Explorer => self.explorer_items.len(),
        };

        if max_len > 0 && self.selected_index + 1 < max_len {
            self.selected_index += 1;
        }
    }

    pub fn switch_tab(&mut self, tab: LibraryTab) {
        self.active_tab = tab;
        self.selected_index = 0;
    }

    pub fn toggle_focus(&mut self) {
        self.focus_panel = match self.focus_panel {
            FocusPanel::Library => FocusPanel::Visualizer,
            FocusPanel::Visualizer => FocusPanel::Controls,
            FocusPanel::Controls => FocusPanel::Library,
        };
    }
}
