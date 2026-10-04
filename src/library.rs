use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::UNIX_EPOCH;
use crossbeam_channel::Receiver;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album_artist: Option<String>,
    pub album: String,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_secs: u64,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub bitrate: Option<u32>,
    pub format_desc: String,
    pub file_size: u64,
    pub modified_time: u64,
}

impl Track {
    pub fn display_title(&self) -> &str {
        if !self.title.trim().is_empty() {
            &self.title
        } else {
            self.path.file_name().and_then(|s| s.to_str()).unwrap_or("Unknown Track")
        }
    }

    pub fn duration_formatted(&self) -> String {
        let mins = self.duration_secs / 60;
        let secs = self.duration_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    pub fn matches_filter(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.title.to_lowercase().contains(&q)
            || self.artist.to_lowercase().contains(&q)
            || self.album.to_lowercase().contains(&q)
            || self.genre.as_ref().map_or(false, |g| g.to_lowercase().contains(&q))
            || self.path.to_string_lossy().to_lowercase().contains(&q)
    }
}

pub fn is_supported_audio_file(path: &Path) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };
    matches!(
        ext.as_str(),
        "flac" | "wav" | "mp3" | "ogg" | "opus" | "aac" | "m4a" | "alac" | "aiff"
    )
}

pub fn parse_track_metadata(path: &Path) -> Option<Track> {
    let metadata = fs::metadata(path).ok()?;
    let file_size = metadata.len();
    let modified_time = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let properties = tagged_file.properties();
    let duration_secs = properties.duration().as_secs();
    let sample_rate = properties.sample_rate();
    let bit_depth = properties.bit_depth();
    let bitrate = properties.audio_bitrate();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| "AUDIO".to_string());

    let mut format_desc = ext.clone();
    if let Some(rate) = sample_rate {
        let khz = rate as f32 / 1000.0;
        if let Some(depth) = bit_depth {
            format_desc = format!("{} {}bit/{:.1}kHz", ext, depth, khz);
        } else if let Some(br) = bitrate {
            format_desc = format!("{} {}kbps/{:.1}kHz", ext, br, khz);
        } else {
            format_desc = format!("{} {:.1}kHz", ext, khz);
        }
    }

    let mut title = None;
    let mut artist = None;
    let album_artist = None;
    let mut album = None;
    let mut genre = None;
    let mut year = None;
    let mut track_number = None;

    if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
        title = tag.title().map(|s| s.to_string());
        artist = tag.artist().map(|s| s.to_string());
        album = tag.album().map(|s| s.to_string());
        genre = tag.genre().map(|s| s.to_string());
        year = tag.year();
        track_number = tag.track();
    }

    let default_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Track")
        .to_string();

    Some(Track {
        path: path.to_path_buf(),
        title: title.unwrap_or(default_title),
        artist: artist.unwrap_or_else(|| "Unknown Artist".to_string()),
        album_artist,
        album: album.unwrap_or_else(|| "Unknown Album".to_string()),
        genre,
        year,
        track_number,
        duration_secs,
        sample_rate,
        bit_depth,
        bitrate,
        format_desc,
        file_size,
        modified_time,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryCache {
    pub tracks: Vec<Track>,
}

pub fn get_cache_path() -> PathBuf {
    if let Some(cache_dir) = dirs::cache_dir() {
        cache_dir.join("apollo/library_cache.json")
    } else {
        PathBuf::from(".apollo_cache.json")
    }
}

pub fn load_cached_library() -> Option<Vec<Track>> {
    let cache_file = get_cache_path();
    if !cache_file.exists() {
        return None;
    }
    let data = fs::read_to_string(&cache_file).ok()?;
    let cache: LibraryCache = serde_json::from_str(&data).ok()?;
    Some(cache.tracks)
}

pub fn save_cached_library(tracks: &[Track]) {
    let cache_file = get_cache_path();
    if let Some(parent) = cache_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let cache = LibraryCache {
        tracks: tracks.to_vec(),
    };
    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = fs::write(cache_file, json);
    }
}

pub enum ScannerMessage {
    Batch(Vec<Track>),
    Finished(usize),
}

pub fn start_background_scanner(
    scan_paths: Vec<PathBuf>,
    cached_tracks: Vec<Track>,
    cancel_flag: Arc<AtomicBool>,
) -> Receiver<ScannerMessage> {
    let (tx, rx) = crossbeam_channel::unbounded();

    thread::spawn(move || {
        let mut known_cache: HashMap<PathBuf, (u64, Track)> = cached_tracks
            .into_iter()
            .map(|t| (t.path.clone(), (t.modified_time, t)))
            .collect();

        let mut discovered_paths = Vec::new();
        for scan_path in &scan_paths {
            if !scan_path.exists() {
                continue;
            }
            for entry in WalkDir::new(scan_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if cancel_flag.load(Ordering::Relaxed) {
                    return;
                }
                let p = entry.path();
                if p.is_file() && is_supported_audio_file(p) {
                    discovered_paths.push(p.to_path_buf());
                }
            }
        }

        let mut final_tracks: Vec<Track> = Vec::new();
        let mut batch = Vec::new();
        let mut new_or_updated = false;

        for path in discovered_paths {
            if cancel_flag.load(Ordering::Relaxed) {
                return;
            }

            let mtime = fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if let Some((cached_mtime, cached_track)) = known_cache.remove(&path) {
                if cached_mtime == mtime {
                    batch.push(cached_track);
                } else if let Some(track) = parse_track_metadata(&path) {
                    new_or_updated = true;
                    batch.push(track);
                }
            } else if let Some(track) = parse_track_metadata(&path) {
                new_or_updated = true;
                batch.push(track);
            }

            if batch.len() >= 50 {
                final_tracks.extend(batch.clone());
                let _ = tx.send(ScannerMessage::Batch(batch));
                batch = Vec::new();
            }
        }

        if !batch.is_empty() {
            final_tracks.extend(batch.clone());
            let _ = tx.send(ScannerMessage::Batch(batch));
        }

        if new_or_updated || !known_cache.is_empty() {
            save_cached_library(&final_tracks);
        }

        let _ = tx.send(ScannerMessage::Finished(final_tracks.len()));
    });

    rx
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryTab {
    Artists,
    Albums,
    Tracks,
    Genres,
    Explorer,
}

impl LibraryTab {
    pub fn title(&self) -> &'static str {
        match self {
            LibraryTab::Artists => "[F1] Wykonawcy",
            LibraryTab::Albums => "[F2] Albumy",
            LibraryTab::Tracks => "[F3] Wszystkie",
            LibraryTab::Genres => "[F4] Gatunki",
            LibraryTab::Explorer => "[F5] Eksplorator",
        }
    }
}

/// Tree item representation for Artists and Genres view
#[derive(Debug, Clone)]
pub enum TreeItem {
    Artist {
        name: String,
        expanded: bool,
        track_count: usize,
    },
    Album {
        artist: String,
        name: String,
        year: Option<u32>,
        expanded: bool,
        track_count: usize,
    },
    TrackLeaf {
        track: Track,
    },
}

