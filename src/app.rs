use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use crossbeam_channel::Receiver;
use serde::{Deserialize, Serialize};
use std::fs;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppState {
    pub volume: f32,
    pub show_visualizer: bool,
    pub active_tab: LibraryTab,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            volume: 1.0,
            show_visualizer: true,
            active_tab: LibraryTab::Artists,
        }
    }
}

pub fn get_state_path() -> PathBuf {
    if let Some(cache_dir) = dirs::cache_dir() {
        cache_dir.join("apollo/app_state.json")
    } else {
        PathBuf::from(".apollo_app_state.json")
    }
}

use crate::audio::{AudioEngine, LoopMode};
use crate::library::{
    is_supported_audio_file, load_cached_library, start_background_scanner,
    LibraryTab, ScannerMessage, Track,
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
pub enum AlbumTreeRow {
    AlbumHeader {
        artist: String,
        album: String,
        year: Option<u32>,
        expanded: bool,
        track_count: usize,
        total_duration_secs: u64,
    },
    TrackItem {
        track: Track,
        is_current: bool,
    },
}

#[derive(Debug, Clone)]
pub enum GenreTreeRow {
    GenreHeader {
        genre: String,
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

    // Albums tab state
    pub expanded_albums_tab: HashSet<String>,

    // Genres tab state
    pub expanded_genres: HashSet<String>,

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

    // Visualizer toggle
    pub show_visualizer: bool,

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

        let state: AppState = fs::read_to_string(get_state_path())
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default();

        let mut audio = AudioEngine::new();
        audio.volume = state.volume;
        audio.prev_volume = state.volume;

        let mut app = Self {
            audio,
            visualizer: AudioVisualizer::new(48000),
            tracks: cached,
            active_tab: state.active_tab,
            focus_panel: FocusPanel::Library,
            selected_index: 0,
            expanded_artists: HashSet::new(),
            expanded_albums: HashSet::new(),
            expanded_albums_tab: HashSet::new(),
            expanded_genres: HashSet::new(),
            explorer_dir: home_music.clone(),
            explorer_items: Vec::new(),
            queue: Vec::new(),
            current_queue_index: 0,
            shuffle_history: Vec::new(),
            is_searching: false,
            search_query: String::new(),
            show_visualizer: state.show_visualizer,
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

    pub fn save_state(&self) {
        let state = AppState {
            volume: self.audio.volume,
            show_visualizer: self.show_visualizer,
            active_tab: self.active_tab,
        };
        let path = get_state_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&state) {
            let _ = fs::write(path, json);
        }
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

    pub fn get_album_tree_rows(&self) -> Vec<AlbumTreeRow> {
        let filtered = self.get_filtered_tracks();
        let current_path = self.audio.current_track.as_ref().map(|t| &t.path);

        let mut map: BTreeMap<String, Vec<Track>> = BTreeMap::new();
        for track in filtered {
            let album_name = if track.album.trim().is_empty() {
                "Nieznany album".to_string()
            } else {
                track.album.clone()
            };
            map.entry(album_name).or_default().push(track);
        }

        let mut rows = Vec::new();
        for (album, mut tracks) in map {
            tracks.sort_by_key(|t| (t.track_number.unwrap_or(0), t.title.clone()));
            let is_album_expanded = self.expanded_albums_tab.contains(&album);
            let year = tracks.iter().find_map(|t| t.year);
            let total_duration_secs = tracks.iter().map(|t| t.duration_secs).sum();
            let count = tracks.len();
            
            // Collect all unique artists for this album
            let mut artists: Vec<String> = tracks.iter().map(|t| t.artist.clone()).collect();
            artists.sort();
            artists.dedup();
            let artist_display = if artists.len() > 1 {
                "Różni wykonawcy".to_string()
            } else if let Some(a) = artists.first() {
                a.clone()
            } else {
                "Nieznany wykonawca".to_string()
            };

            rows.push(AlbumTreeRow::AlbumHeader {
                artist: artist_display,
                album: album.clone(),
                year,
                expanded: is_album_expanded,
                track_count: count,
                total_duration_secs,
            });

            if is_album_expanded {
                for track in tracks {
                    let is_current = current_path == Some(&track.path);
                    rows.push(AlbumTreeRow::TrackItem { track, is_current });
                }
            }
        }
        rows
    }


    pub fn get_genre_tree_rows(&self) -> Vec<GenreTreeRow> {
        let filtered = self.get_filtered_tracks();
        let current_path = self.audio.current_track.as_ref().map(|t| &t.path);

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

        let mut rows = Vec::new();
        for (genre, mut tracks) in map {
            tracks.sort_by_key(|t| (t.artist.clone(), t.album.clone(), t.track_number.unwrap_or(0), t.title.clone()));
            let is_expanded = self.expanded_genres.contains(&genre);
            let count = tracks.len();

            rows.push(GenreTreeRow::GenreHeader {
                genre: genre.clone(),
                expanded: is_expanded,
                track_count: count,
            });

            if is_expanded {
                for track in tracks {
                    let is_current = current_path == Some(&track.path);
                    rows.push(GenreTreeRow::TrackItem { track, is_current });
                }
            }
        }
        rows
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
                            let mut album_tracks: Vec<Track> = self
                                .tracks
                                .iter()
                                .filter(|t| t.artist == track.artist && t.album == track.album)
                                .cloned()
                                .collect();
                            
                            album_tracks.sort_by_key(|t| (t.track_number.unwrap_or(0), t.title.clone()));

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
                let rows = self.get_album_tree_rows();
                if let Some(row) = rows.get(self.selected_index) {
                    match row {
                        AlbumTreeRow::AlbumHeader { album, .. } => {
                            if self.expanded_albums_tab.contains(album) {
                                self.expanded_albums_tab.remove(album);
                            } else {
                                self.expanded_albums_tab.insert(album.clone());
                            }
                        }
                        AlbumTreeRow::TrackItem { track, .. } => {
                            let mut album_tracks: Vec<Track> = self
                                .tracks
                                .iter()
                                .filter(|t| t.album == track.album)
                                .cloned()
                                .collect();

                            album_tracks.sort_by_key(|t| (t.track_number.unwrap_or(0), t.title.clone()));

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
            LibraryTab::Tracks => {
                let tracks = self.get_filtered_tracks();
                if tracks.get(self.selected_index).is_some() {
                    self.queue = tracks;
                    self.play_track_at_queue_index(self.selected_index);
                }
            }
            LibraryTab::Genres => {
                let rows = self.get_genre_tree_rows();
                if let Some(row) = rows.get(self.selected_index) {
                    match row {
                        GenreTreeRow::GenreHeader { genre, .. } => {
                            if self.expanded_genres.contains(genre) {
                                self.expanded_genres.remove(genre);
                            } else {
                                self.expanded_genres.insert(genre.clone());
                            }
                        }
                        GenreTreeRow::TrackItem { track, .. } => {
                            let mut genre_tracks: Vec<Track> = self
                                .tracks
                                .iter()
                                .filter(|t| {
                                    t.genre.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("Nieokreślony") ==
                                        track.genre.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("Nieokreślony")
                                })
                                .cloned()
                                .collect();

                            genre_tracks.sort_by_key(|t| (t.artist.clone(), t.album.clone(), t.track_number.unwrap_or(0), t.title.clone()));

                            let queue = if !genre_tracks.is_empty() {
                                genre_tracks
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
        } else {
            let max_len = match self.active_tab {
                LibraryTab::Artists => self.get_artist_tree_rows().len(),
                LibraryTab::Albums => self.get_album_tree_rows().len(),
                LibraryTab::Tracks => self.get_filtered_tracks().len(),
                LibraryTab::Genres => self.get_genre_tree_rows().len(),
                LibraryTab::Explorer => self.explorer_items.len(),
            };
            if max_len > 0 {
                self.selected_index = max_len - 1;
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        let max_len = match self.active_tab {
            LibraryTab::Artists => self.get_artist_tree_rows().len(),
            LibraryTab::Albums => self.get_album_tree_rows().len(),
            LibraryTab::Tracks => self.get_filtered_tracks().len(),
            LibraryTab::Genres => self.get_genre_tree_rows().len(),
            LibraryTab::Explorer => self.explorer_items.len(),
        };

        if max_len > 0 {
            if self.selected_index + 1 < max_len {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
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
