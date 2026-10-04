use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, Paragraph, Row, Table, Wrap,
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
        let msg = Paragraph::new(app.i18n.t("term_too_small"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, size);
        return;
    }


    let show_logo = app.config.show_logo && size.width >= 50 && size.height >= 25; // Potrzeba więcej miejsca w pionie
    let (vis_height, can_show_vis) = if size.height >= 26 {
        (8, true)
    } else if size.height >= 20 {
        (5, true)
    } else {
        (0, false)
    };
    let show_vis = can_show_vis && app.show_visualizer;

    let mut constraints = vec![];
    if show_logo {
        constraints.push(Constraint::Length(5));       // Logo
    }
    constraints.push(Constraint::Min(8));              // Library panel
    if show_vis {
        constraints.push(Constraint::Length(vis_height)); // Visualizer
    }
    constraints.push(Constraint::Length(5));           // Now Playing
    
    let footer_height = if app.show_hotkeys {
        (155 + size.width.saturating_sub(1)) / size.width.max(1)
    } else {
        0
    };
    constraints.push(Constraint::Length(footer_height)); // Hotkeys footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let mut chunk_idx = 0;
    let logo_rect = if show_logo {
        let rect = chunks[chunk_idx];
        chunk_idx += 1;
        Some(rect)
    } else {
        None
    };

    let library_rect = chunks[chunk_idx];
    chunk_idx += 1;

    let vis_rect = if show_vis {
        let rect = chunks[chunk_idx];
        chunk_idx += 1;
        Some(rect)
    } else {
        None
    };

    let now_playing_rect = chunks[chunk_idx];
    chunk_idx += 1;
    let footer_rect = chunks[chunk_idx];

    // 1. Render Library Panel
    render_library_panel(frame, app, library_rect);

    // 2. Render Visualizer Panel (if space permits)
    if let Some(v_rect) = vis_rect {
        render_visualizer_panel(frame, app, v_rect);
    }

    // 3. Render Now Playing Panel
    render_now_playing_panel(frame, app, now_playing_rect, show_vis);

    // 4. Render Footer Hotkey / Search bar
    render_footer(frame, app, footer_rect);

    // 5. Render ASCII art logo if there is enough space
    if let Some(rect) = logo_rect {
        let art = vec![
            Line::from("    _             _ _     "),
            Line::from("   /_\\  _ __  ___| | |___ "),
            Line::from("  / _ \\| '_ \\/ _ \\ | / _ \\"),
            Line::from(" /_/ \\_\\ .__/\\___/_|_\\___/"),
            Line::from("       |_|                "),
        ];
        let art_widget = Paragraph::new(art)
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        
        frame.render_widget(art_widget, rect);
    }
}

fn render_library_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus_panel == FocusPanel::Library;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    // Tabs headers
    let tab_titles = vec![
        app.i18n.t("library_tab_artists"),
        app.i18n.t("library_tab_albums"),
        app.i18n.t("library_tab_tracks"),
        app.i18n.t("library_tab_genres"),
        app.i18n.t("library_tab_explorer"),
    ];

    let selected_tab_idx = match app.active_tab {
        LibraryTab::Artists => 0,
        LibraryTab::Albums => 1,
        LibraryTab::Tracks => 2,
        LibraryTab::Genres => 3,
        LibraryTab::Explorer => 4,
    };

    let mut tab_spans = vec![
        Span::styled(
            format!(" 1. {} ", app.i18n.t("library_title").to_uppercase()),
            Style::default().fg(if is_focused { Color::Cyan } else { Color::Reset }).add_modifier(Modifier::BOLD)
        ),
    ];

    for (i, title) in tab_titles.iter().enumerate() {
        let label = format!(" F{} - {} ", i + 1, title);
        if i == selected_tab_idx {
            tab_spans.push(Span::styled(
                label,
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
            ));
        } else {
            tab_spans.push(Span::styled(
                label,
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            ));
        }
        if i < tab_titles.len() - 1 {
            tab_spans.push(Span::raw(" "));
        }
    }

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(tab_spans));

    frame.render_widget(outer_block, area);

    // Inner layout for Content
    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(4),
    };

    if content_area.height < 1 {
        return;
    }

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
            app.i18n.t("scanning")
        } else {
            app.i18n.t("no_tracks")
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
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };

            match row {
                ArtistTreeRow::ArtistHeader {
                    artist,
                    expanded,
                    album_count,
                    track_count,
                } => {
                    let icon = if *expanded { " v " } else { " > " };
                    let text = format!("{}{} ({} {}, {} {})", icon, artist, album_count, app.i18n.t("albums_count"), track_count, app.i18n.pluralize_tracks(*track_count));
                    let style = if is_selected {
                        base_style
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
                    let icon = if *expanded { "   v " } else { "   > " };
                    let yr_str = year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    let text = format!("{}{}{} [{} {}]", icon, album, yr_str, track_count, app.i18n.pluralize_tracks(*track_count));
                    let style = if is_selected {
                        base_style
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
                            base_style
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
        let msg = Paragraph::new(app.i18n.t("no_albums"))
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
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
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
                    let icon = if *expanded { " v " } else { " > " };
                    let yr_str = year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    let text = format!("{}{}{} - {} [{} {}]", icon, artist, yr_str, album, track_count, app.i18n.pluralize_tracks(*track_count));
                    let style = if is_selected {
                        base_style
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
                            base_style
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
        let msg = Paragraph::new(app.i18n.t("no_filtered_tracks"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        app.i18n.t("col_artist"),
        app.i18n.t("col_title"),
        app.i18n.t("col_album"),
        app.i18n.t("col_time"),
        app.i18n.t("col_format"),
    ])
    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let current_path = app.audio.current_track.as_ref().map(|t| &t.path);

    let rows: Vec<Row> = tracks
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            let is_selected = idx == app.selected_index;
            let is_current = current_path == Some(&t.path);

            let style = if is_selected {
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };

            let prefix = if is_current { "* " } else { "  " };
            let title_display = format!("{}{}", prefix, t.display_title());

            Row::new(vec![
                format!(" {}", t.artist),
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
        let msg = Paragraph::new(app.i18n.t("no_genres"))
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
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };

            match row {
                crate::app::GenreTreeRow::GenreHeader { genre, expanded, track_count } => {
                    let icon = if *expanded { " v " } else { " > " };
                    let text = format!("{}{:<25} [{} {}]", icon, genre, track_count, app.i18n.pluralize_tracks(*track_count));
                    let style = if is_selected {
                        base_style
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
                            base_style
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
                Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else if item.is_dir {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };

            let prefix = if item.is_dir { " [DIR] " } else { " [MUS] " };
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

    let title_line = if app.show_hotkeys {
        Line::from(vec![
            Span::styled(format!(" 2. {} ", app.i18n.t("visualizer_title").to_uppercase()), Style::default().fg(if is_focused { Color::Cyan } else { Color::Reset }).add_modifier(Modifier::BOLD)),
            Span::styled(if app.config.language == crate::config::Language::Polish { " V - Przełącz " } else { " V - Toggle " }, Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(if app.config.language == crate::config::Language::Polish { " C - Pokaż/Ukryj " } else { " C - Show/Hide " }, Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!(" 2. {} ", app.i18n.t("visualizer_title").to_uppercase()), Style::default().fg(if is_focused { Color::Cyan } else { Color::Reset }).add_modifier(Modifier::BOLD)),
        ])
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title_line);

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

fn render_spectrum(frame: &mut Frame, app: &mut App, mut area: Rect) {
    area.x += 1;
    area.width = area.width.saturating_sub(2);
    
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
                let col = match app.config.visualizer_color {
                    crate::config::VisualizerColorMode::Rainbow => {
                        let colors = [Color::Red, Color::Yellow, Color::Green, Color::Cyan, Color::Blue, Color::Magenta];
                        colors[b_idx % colors.len()]
                    }
                    crate::config::VisualizerColorMode::Solid => Color::Cyan,
                };
                (UNICODE_BLOCKS[8], col)
            } else if total_sublevels > row_min_sub {
                let sub = total_sublevels - row_min_sub;
                let col = match app.config.visualizer_color {
                    crate::config::VisualizerColorMode::Rainbow => {
                        let colors = [Color::Red, Color::Yellow, Color::Green, Color::Cyan, Color::Blue, Color::Magenta];
                        colors[b_idx % colors.len()]
                    }
                    crate::config::VisualizerColorMode::Solid => Color::Cyan,
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
        match app.config.visualizer_color {
            crate::config::VisualizerColorMode::Solid => {
                let row_str: String = grid[row].iter().collect();
                lines.push(Line::from(Span::styled(
                    row_str,
                    Style::default().fg(Color::Cyan),
                )));
            }
            crate::config::VisualizerColorMode::Rainbow => {
                let colors = [Color::Red, Color::Yellow, Color::Green, Color::Cyan, Color::Blue, Color::Magenta];
                let mut spans = Vec::with_capacity(width);
                for (x, &ch) in grid[row].iter().enumerate() {
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(colors[x % colors.len()]),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }
    }

    let p = Paragraph::new(lines);
    frame.render_widget(p, area);
}

fn render_now_playing_panel(frame: &mut Frame, app: &App, area: Rect, is_visualizer_shown: bool) {
    let is_focused = app.focus_panel == FocusPanel::Controls;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let panel_num = if is_visualizer_shown { 3 } else { 2 };
    
    let title = format!(" {}. {} ", panel_num, app.i18n.t("now_playing_title"));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default().fg(if is_focused { Color::Cyan } else { Color::Reset }).add_modifier(Modifier::BOLD),
        ));

    let mut inner = block.inner(area);
    inner.x += 1;
    inner.width = inner.width.saturating_sub(2);
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
    let state_str = if app.audio.is_playing {
        app.i18n.t("play_state_playing")
    } else if current.is_some() {
        app.i18n.t("play_state_paused")
    } else {
        app.i18n.t("play_state_stopped")
    };
    
    let state_indicator = if app.audio.is_playing {
        Span::styled(state_str, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else if current.is_some() {
        Span::styled(state_str, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(state_str, Style::default().fg(Color::DarkGray))
    };

    let track_info = if let Some(t) = current {
        format!("{} - {}", t.artist, t.display_title())
    } else {
        app.i18n.t("no_active_track").to_string()
    };

    // Volume bar
    let vol_percent = (app.audio.volume * 100.0).round() as u32;
    let vol_str = if app.audio.is_muted {
        app.i18n.t("volume_muted").to_string()
    } else {
        let filled_bars = (vol_percent / 10).min(10) as usize;
        let empty_bars = 10 - filled_bars;
        format!("{}[{}{}] {}%", app.i18n.t("volume"), "█".repeat(filled_bars), "░".repeat(empty_bars), vol_percent)
    };

    let left_len = state_str.chars().count() + track_info.chars().count();
    let vol_len = vol_str.chars().count();
    let padding_len = (inner.width as usize).saturating_sub(left_len + vol_len);
    let padding_str = " ".repeat(padding_len);

    let line1 = Line::from(vec![
        state_indicator,
        Span::styled(track_info, Style::default().fg(Color::Reset).add_modifier(Modifier::BOLD)),
        Span::styled(padding_str, Style::default()),
        Span::styled(vol_str, Style::default().fg(Color::Cyan)),
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
        Span::styled(progress_bar_str, Style::default().fg(Color::Reset)),
        Span::styled(time_suffix, Style::default().fg(Color::Cyan)),
    ]);

    // Metadata & Status: [Zapętlenie: Utwór] | [Mieszanie: WŁ] | Kolejka: 2/10 | Format: FLAC 96kHz 24-bit
    let loop_status = format!("[{}: {}]", app.i18n.t("loop_status"), app.i18n.t(app.audio.loop_mode.display_label()));
    let shuffle_status = format!("[{}: {}]", app.i18n.t("shuffle_status"), if app.audio.shuffle { app.i18n.t("on") } else { app.i18n.t("off") });
    let queue_status = format!(
        "{}: {}/{}",
        app.i18n.t("queue_status"),
        if app.queue.is_empty() { 0 } else { app.current_queue_index + 1 },
        app.queue.len()
    );
    let format_status = current
        .map(|t| format!("{}: {}", app.i18n.t("format_status"), t.format_desc))
        .unwrap_or_else(|| format!("{}: -", app.i18n.t("format_status")));

    let mut line3_spans = vec![
        Span::styled(loop_status, Style::default().fg(if app.audio.loop_mode != LoopMode::Off { Color::Green } else { Color::DarkGray })),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(shuffle_status, Style::default().fg(if app.audio.shuffle { Color::Green } else { Color::DarkGray })),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(queue_status, Style::default().fg(Color::Reset)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(format_status, Style::default().fg(Color::Yellow)),
    ];

    if let Some(status) = app.get_status() {
        line3_spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        line3_spans.push(Span::styled("[i]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        line3_spans.push(Span::styled(format!(" {}", status), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    }

    let p = Paragraph::new(vec![line1, line2, Line::from(line3_spans)]);
    frame.render_widget(p, inner);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    if app.is_searching {
        let search_text = format!("[ {}: {}_ ] ({})", app.i18n.t("search_footer"), app.search_query, app.i18n.t("search_cancel_hint"));
        let p = Paragraph::new(search_text)
            .style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD));
        frame.render_widget(p, area);
        return;
    }

    let mut lines = Vec::new();

        let hotkeys = Line::from(vec![
        Span::styled(format!(" Spacja - {} ", app.i18n.t("hk_play")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" Z / X - {} ", app.i18n.t("hk_prev_next")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" < / > - {} ", app.i18n.t("hk_seek")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" S - {} ", app.i18n.t("hk_shuffle")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" L - {} ", app.i18n.t("hk_loop")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" + / - - {} ", app.i18n.t("hk_volume")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" / - {} ", app.i18n.t("hk_search")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(if app.config.language == crate::config::Language::Polish { " H - Ukryj Skróty " } else { " H - Hide Hotkeys " }, Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!(" Q - {} ", app.i18n.t("hk_quit")), Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
    ]);
    lines.push(hotkeys);

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}
