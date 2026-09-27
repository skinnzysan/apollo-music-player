use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, Paragraph, Row, Table, Tabs,
    },
    Frame,
};

use crate::app::{App, ArtistTreeRow, FocusPanel};
use crate::audio::LoopMode;
use crate::library::LibraryTab;
use crate::visualizer::VisualizerMode;

const UNICODE_BLOCKS: [char; 9] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn draw_ui(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Check minimum dimensions
    if size.width < 50 || size.height < 14 {
        let msg = Paragraph::new("Zbyt mały rozmiar terminala (wymagane min. 50x14)")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, size);
        return;
    }

    // Determine layout constraints based on terminal height
    let (vis_height, show_vis) = if size.height >= 26 {
        (8, true)
    } else if size.height >= 20 {
        (5, true)
    } else {
        (0, false)
    };

    let constraints = if show_vis {
        vec![
            Constraint::Min(8),            // Library panel
            Constraint::Length(vis_height), // Visualizer
            Constraint::Length(5),         // Now Playing
            Constraint::Length(1),         // Hotkeys / status footer
        ]
    } else {
        vec![
            Constraint::Min(8),            // Library panel
            Constraint::Length(5),         // Now Playing
            Constraint::Length(1),         // Hotkeys / status footer
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let library_rect = chunks[0];
    let (vis_rect, now_playing_rect, footer_rect) = if show_vis {
        (Some(chunks[1]), chunks[2], chunks[3])
    } else {
        (None, chunks[1], chunks[2])
    };

    // 1. Render Library Panel
    render_library_panel(frame, app, library_rect);

    // 2. Render Visualizer Panel (if space permits)
    if let Some(v_rect) = vis_rect {
        render_visualizer_panel(frame, app, v_rect);
    }

    // 3. Render Now Playing Panel
    render_now_playing_panel(frame, app, now_playing_rect);

    // 4. Render Footer Hotkey / Search bar
    render_footer(frame, app, footer_rect);
}

fn render_library_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus_panel == FocusPanel::Library;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    // Tabs headers
    let tab_titles = vec![
        "[F1] Wykonawcy",
        "[F2] Albumy",
        "[F3] Wszystkie",
        "[F4] Gatunki",
        "[F5] Eksplorator",
    ];

    let selected_tab_idx = match app.active_tab {
        LibraryTab::Artists => 0,
        LibraryTab::Albums => 1,
        LibraryTab::Tracks => 2,
        LibraryTab::Genres => 3,
        LibraryTab::Explorer => 4,
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " 1. BIBLIOTEKA MUZYCZNA ",
            Style::default().fg(if is_focused { Color::Cyan } else { Color::White }).add_modifier(Modifier::BOLD),
        ));

    frame.render_widget(outer_block, area);

    // Inner layout for Tabs and Content
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner_area.height < 2 {
        return;
    }

    let tab_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner_area);

    let tabs = Tabs::new(tab_titles)
        .select(selected_tab_idx)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .divider(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

    frame.render_widget(tabs, tab_layout[0]);

    let content_area = tab_layout[1];

    match app.active_tab {
        LibraryTab::Artists => render_artist_tree(frame, app, content_area),
        LibraryTab::Albums => render_albums_view(frame, app, content_area),
        LibraryTab::Tracks => render_all_tracks_view(frame, app, content_area),
        LibraryTab::Genres => render_genres_view(frame, app, content_area),
        LibraryTab::Explorer => render_explorer_view(frame, app, content_area),
    }
}

fn render_artist_tree(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.get_artist_tree_rows();
    if rows.is_empty() {
        let msg = Paragraph::new(if app.is_scanning {
            "Skanowanie katalogu ~/Music w toku..."
        } else {
            "Brak utworów w bibliotece. Umieść pliki audio w ~/Music"
        })
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let is_selected = idx == app.selected_index;
            let base_style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            match row {
                ArtistTreeRow::ArtistHeader {
                    artist,
                    expanded,
                    album_count,
                    track_count,
                } => {
                    let icon = if *expanded { "v " } else { "> " };
                    let text = format!("{}{} ({} albumy, {} utw.)", icon, artist, album_count, track_count);
                    let style = if is_selected {
                        base_style.fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    };
                    ListItem::new(Span::styled(text, style))
                }
                ArtistTreeRow::AlbumHeader {
                    album,
                    year,
                    expanded,
                    track_count,
                    ..
                } => {
                    let icon = if *expanded { "  v " } else { "  > " };
                    let yr_str = year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    let text = format!("{}{}{} [{} utw.]", icon, album, yr_str, track_count);
                    let style = if is_selected {
                        base_style.fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    ListItem::new(Span::styled(text, style))
                }
                ArtistTreeRow::TrackItem { track, is_current } => {
                    let marker = if *is_current { " * " } else { "   " };
                    let trk_no = track
                        .track_number
                        .map(|n| format!("{:02}. ", n))
                        .unwrap_or_else(|| "    ".to_string());
                    let dur = track.duration_formatted();
                    let title = track.display_title();
                    let fmt = &track.format_desc;

                    let line_text = format!(
                        "    {}{}{:<35} | {:<5} | {:<20}",
                        marker, trk_no, title, dur, fmt
                    );

                    let style = if *is_current {
                        if is_selected {
                            base_style.fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        }
                    } else {
                        base_style
                    };

                    ListItem::new(Span::styled(line_text, style))
                }
            }
        })
        .collect();

    let list = List::new(items);
    render_scrollable_list(frame, list, area, app.selected_index, rows.len());
}

fn render_albums_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.get_album_tree_rows();
    if rows.is_empty() {
        let msg = Paragraph::new("Brak albumów.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let is_selected = idx == app.selected_index;
            let base_style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            match row {
                crate::app::AlbumTreeRow::AlbumHeader {
                    artist,
                    album,
                    year,
                    expanded,
                    track_count,
                    ..
                } => {
                    let icon = if *expanded { "v " } else { "> " };
                    let yr_str = year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    let text = format!("{}{}{} - {} [{} utw.]", icon, artist, yr_str, album, track_count);
                    let style = if is_selected {
                        base_style.fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    };
                    ListItem::new(Span::styled(text, style))
                }
                crate::app::AlbumTreeRow::TrackItem { track, is_current } => {
                    let marker = if *is_current { " * " } else { "   " };
                    let trk_no = track
                        .track_number
                        .map(|n| format!("{:02}. ", n))
                        .unwrap_or_else(|| "    ".to_string());
                    let dur = track.duration_formatted();
                    let title = track.display_title();
                    let fmt = &track.format_desc;

                    let line_text = format!(
                        "    {}{}{:<35} | {:<5} | {:<20}",
                        marker, trk_no, title, dur, fmt
                    );

                    let style = if *is_current {
                        if is_selected {
                            base_style.fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        }
                    } else {
                        base_style
                    };

                    ListItem::new(Span::styled(line_text, style))
                }
            }
        })
        .collect();

    let list = List::new(items);
    render_scrollable_list(frame, list, area, app.selected_index, rows.len());
}

fn render_all_tracks_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let tracks = app.get_filtered_tracks();
    if tracks.is_empty() {
        let msg = Paragraph::new("Brak utworów spełniających kryteria.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["Wykonawca", "Tytuł", "Album", "Czas", "Format"])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let current_path = app.audio.current_track.as_ref().map(|t| &t.path);

    let rows: Vec<Row> = tracks
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            let is_selected = idx == app.selected_index;
            let is_current = current_path == Some(&t.path);

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(if is_current { Color::Green } else { Color::White }).add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_current { "* " } else { "  " };
            let title_display = format!("{}{}", prefix, t.display_title());

            Row::new(vec![
                t.artist.clone(),
                title_display,
                t.album.clone(),
                t.duration_formatted(),
                t.format_desc.clone(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(30),
        Constraint::Percentage(25),
        Constraint::Length(7),
        Constraint::Length(18),
    ];

    let table = Table::new(rows, widths).header(header);
    render_scrollable_table(frame, table, area, app.selected_index, tracks.len());
}

fn render_genres_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.get_genre_tree_rows();
    if rows.is_empty() {
        let msg = Paragraph::new("Brak gatunków.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let is_selected = idx == app.selected_index;
            let base_style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            match row {
                crate::app::GenreTreeRow::GenreHeader { genre, expanded, track_count } => {
                    let icon = if *expanded { "v " } else { "> " };
                    let text = format!("{}{:<25} [{} utw.]", icon, genre, track_count);
                    let style = if is_selected {
                        base_style.fg(Color::Magenta)
                    } else {
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                    };
                    ListItem::new(Span::styled(text, style))
                }
                crate::app::GenreTreeRow::TrackItem { track, is_current } => {
                    let marker = if *is_current { " * " } else { "   " };
                    let dur = track.duration_formatted();
                    let title = track.display_title();
                    let artist = &track.artist;

                    let line_text = format!(
                        "    {}{:<25} - {:<30} | {:<5}",
                        marker, artist, title, dur
                    );

                    let style = if *is_current {
                        if is_selected {
                            base_style.fg(Color::Green)
                        } else {
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        }
                    } else {
                        base_style
                    };

                    ListItem::new(Span::styled(line_text, style))
                }
            }
        })
        .collect();

    let list = List::new(items);
    render_scrollable_list(frame, list, area, app.selected_index, rows.len());
}

fn render_explorer_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .explorer_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_index;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            } else if item.is_dir {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if item.is_dir { "[DIR] " } else { "[MUS] " };
            ListItem::new(Span::styled(format!("{}{}", prefix, item.name), style))
        })
        .collect();

    let list = List::new(items);
    render_scrollable_list(frame, list, area, app.selected_index, app.explorer_items.len());
}

fn render_scrollable_list(frame: &mut Frame, list: List, area: Rect, selected: usize, total: usize) {
    if total == 0 {
        return;
    }
    let height = area.height as usize;
    let offset = if selected >= height {
        selected.saturating_sub(height - 1)
    } else {
        0
    };

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected));
    *state.offset_mut() = offset;
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_scrollable_table(frame: &mut Frame, table: Table, area: Rect, selected: usize, total: usize) {
    if total == 0 {
        return;
    }
    let height = area.height.saturating_sub(1) as usize; // account for header
    let offset = if height > 0 && selected >= height {
        selected.saturating_sub(height - 1)
    } else {
        0
    };

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(selected));
    *state.offset_mut() = offset;
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_visualizer_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus_panel == FocusPanel::Visualizer;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let mode_str = match app.visualizer.mode {
        VisualizerMode::Spectrum => "SPECTRUM (FFT)",
        VisualizerMode::Waveform => "WAVEFORM (PCM)",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" 2. WIZUALIZATOR AUDIO ({}) [v - Przełącz] ", mode_str),
            Style::default().fg(if is_focused { Color::Cyan } else { Color::White }).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 1 {
        return;
    }

    match app.visualizer.mode {
        VisualizerMode::Spectrum => render_spectrum(frame, app, inner),
        VisualizerMode::Waveform => render_waveform(frame, app, inner),
    }
}

fn render_spectrum(frame: &mut Frame, app: &mut App, area: Rect) {
    let width = area.width as usize;
    let height = area.height as usize;
    if width == 0 || height == 0 {
        return;
    }

    // Number of bars: 1 bar per 2 characters (bar + space) or dense
    let bar_width = 2;
    let num_bars = width / bar_width;
    if num_bars == 0 {
        return;
    }

    let bar_values = app.visualizer.compute_spectrum(
        &app.audio.samples_buffer,
        num_bars,
        app.audio.is_playing,
    );

    // Build grid of characters for spectrum display
    let mut grid: Vec<Vec<(char, Color)>> = vec![vec![(' ', Color::Reset); width]; height];

    for (b_idx, &val) in bar_values.iter().enumerate() {
        let total_sublevels = (val * (height as f32 * 8.0)).round() as usize;
        let col_start = b_idx * bar_width;

        for row in 0..height {
            let row_from_bottom = (height - 1) - row;
            let row_min_sub = row_from_bottom * 8;

            let (ch, color) = if total_sublevels >= row_min_sub + 8 {
                let col = if row_from_bottom as f32 / height as f32 > 0.8 {
                    Color::Red
                } else if row_from_bottom as f32 / height as f32 > 0.5 {
                    Color::Yellow
                } else {
                    Color::Cyan
                };
                (UNICODE_BLOCKS[8], col)
            } else if total_sublevels > row_min_sub {
                let sub = total_sublevels - row_min_sub;
                let col = if row_from_bottom as f32 / height as f32 > 0.8 {
                    Color::Red
                } else if row_from_bottom as f32 / height as f32 > 0.5 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                (UNICODE_BLOCKS[sub.min(8)], col)
            } else {
                (' ', Color::Reset)
            };

            for w in 0..bar_width.saturating_sub(1) {
                if col_start + w < width {
                    grid[row][col_start + w] = (ch, color);
                }
            }
        }
    }

    // Render grid lines
    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let spans: Vec<Span> = grid[row]
            .iter()
            .map(|&(ch, color)| {
                Span::styled(ch.to_string(), Style::default().fg(color))
            })
            .collect();
        lines.push(Line::from(spans));
    }

    let p = Paragraph::new(lines);
    frame.render_widget(p, area);
}

fn render_waveform(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width as usize;
    let height = area.height as usize;
    if width == 0 || height == 0 {
        return;
    }

    let points = app.visualizer.get_waveform_points(&app.audio.samples_buffer, width);
    let mid_y = height / 2;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; width]; height];

    for x in 0..width {
        let val = points.get(x).copied().unwrap_or(0.0);
        let y_offset = (val * (height as f32 / 2.0)).round() as i32;
        let y = ((mid_y as i32) - y_offset).clamp(0, (height - 1) as i32) as usize;
        grid[y][x] = '~';
    }

    let mut lines = Vec::with_capacity(height);
    for row in 0..height {
        let row_str: String = grid[row].iter().collect();
        lines.push(Line::from(Span::styled(
            row_str,
            Style::default().fg(Color::Cyan),
        )));
    }

    let p = Paragraph::new(lines);
    frame.render_widget(p, area);
}

fn render_now_playing_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus_panel == FocusPanel::Controls;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " 3. TERAZ ODTWARZANE ",
            Style::default().fg(if is_focused { Color::Cyan } else { Color::White }).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    let current = app.audio.current_track.as_ref();
    let current_pos = app.audio.get_position();
    let total_dur_secs = current.map(|t| t.duration_secs).unwrap_or(0);

    let pos_formatted = format!("{:02}:{:02}", current_pos.as_secs() / 60, current_pos.as_secs() % 60);
    let total_formatted = format!("{:02}:{:02}", total_dur_secs / 60, total_dur_secs % 60);

    let progress_ratio = if total_dur_secs > 0 {
        (current_pos.as_secs_f64() / total_dur_secs as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Playback state indicator
    let state_indicator = if app.audio.is_playing {
        Span::styled("[>] Odtwarzanie: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else if current.is_some() {
        Span::styled("[||] Wstrzymano: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[■] Zatrzymano: ", Style::default().fg(Color::DarkGray))
    };

    let track_info = if let Some(t) = current {
        format!("{} - {}", t.artist, t.display_title())
    } else {
        "Brak aktywnego utworu".to_string()
    };

    // Volume bar
    let vol_percent = (app.audio.volume * 100.0).round() as u32;
    let vol_str = if app.audio.is_muted {
        "Głośność: [WYCISZONO]".to_string()
    } else {
        let filled_bars = (vol_percent / 10).min(10) as usize;
        let empty_bars = 10 - filled_bars;
        format!("Głośność: [{}{}] {}%", "█".repeat(filled_bars), "░".repeat(empty_bars), vol_percent)
    };

    let line1 = Line::from(vec![
        state_indicator,
        Span::styled(track_info, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {:>width$}", vol_str, width = inner.width.saturating_sub(50) as usize), Style::default().fg(Color::Cyan)),
    ]);

    // Progress bar line: 01:14 [================--------] 02:49
    let time_prefix = format!("{} ", pos_formatted);
    let time_suffix = format!(" {}", total_formatted);
    let bar_width = (inner.width as usize).saturating_sub(time_prefix.len() + time_suffix.len() + 2);

    let filled_len = ((progress_ratio * bar_width as f64).round() as usize).min(bar_width);
    let empty_len = bar_width.saturating_sub(filled_len);

    let progress_bar_str = format!("[{}{}]", "=".repeat(filled_len), "-".repeat(empty_len));

    let line2 = Line::from(vec![
        Span::styled(time_prefix, Style::default().fg(Color::Cyan)),
        Span::styled(progress_bar_str, Style::default().fg(Color::White)),
        Span::styled(time_suffix, Style::default().fg(Color::Cyan)),
    ]);

    // Metadata & Status: [Zapętlenie: Utwór] | [Mieszanie: WŁ] | Kolejka: 2/10 | Format: FLAC 96kHz 24-bit
    let loop_status = format!("[Zapętlenie: {}]", app.audio.loop_mode.display_label());
    let shuffle_status = format!("[Mieszanie: {}]", if app.audio.shuffle { "WŁ" } else { "WYŁ" });
    let queue_status = format!(
        "Kolejka: {}/{}",
        if app.queue.is_empty() { 0 } else { app.current_queue_index + 1 },
        app.queue.len()
    );
    let format_status = current
        .map(|t| format!("Format: {}", t.format_desc))
        .unwrap_or_else(|| "Format: -".to_string());

    let line3 = Line::from(vec![
        Span::styled(loop_status, Style::default().fg(if app.audio.loop_mode != LoopMode::Off { Color::Green } else { Color::DarkGray })),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(shuffle_status, Style::default().fg(if app.audio.shuffle { Color::Green } else { Color::DarkGray })),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(queue_status, Style::default().fg(Color::White)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(format_status, Style::default().fg(Color::Yellow)),
    ]);

    let p = Paragraph::new(vec![line1, line2, line3]);
    frame.render_widget(p, inner);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    if app.is_searching {
        let search_text = format!("[ Wyszukaj: {}_ ] (Naciśnij Esc aby anulować, Enter aby zatwierdzić)", app.search_query);
        let p = Paragraph::new(search_text)
            .style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD));
        frame.render_widget(p, area);
        return;
    }

    if let Some(status) = app.get_status() {
        let p = Paragraph::new(format!("  [i] {}", status))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(p, area);
        return;
    }

    let hotkeys = Line::from(vec![
        Span::styled("[Spacja]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Pauza/Play | ", Style::default().fg(Color::White)),
        Span::styled("[n]/[p]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Nast/Poprz | ", Style::default().fg(Color::White)),
        Span::styled("[h]/[l]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Skok ±5s | ", Style::default().fg(Color::White)),
        Span::styled("[s]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Shuffle | ", Style::default().fg(Color::White)),
        Span::styled("[r]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Pętla | ", Style::default().fg(Color::White)),
        Span::styled("[+/-]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Głośność | ", Style::default().fg(Color::White)),
        Span::styled("[/]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Szukaj | ", Style::default().fg(Color::White)),
        Span::styled("[q]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" Wyjście", Style::default().fg(Color::White)),
    ]);

    let p = Paragraph::new(hotkeys);
    frame.render_widget(p, area);
}
