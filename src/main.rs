pub mod app;
pub mod audio;
pub mod library;
pub mod ui;
pub mod visualizer;
pub mod config;
pub mod i18n;

use std::env;
use std::io::{self, stdout};
use std::panic;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::library::LibraryTab;
use crate::ui::draw_ui;

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

fn main() -> io::Result<()> {
    setup_panic_hook();

    // Determine initial music directories
    let args: Vec<String> = env::args().skip(1).collect();

    for arg in &args {
        if arg == "-h" || arg == "--help" {
            println!("Apollo - Terminal Music Player for Linux");
            println!();
            println!("USAGE:");
            println!("    apollo [PATH...]     Start player and set PATH as the new default library in config");
            println!();
            println!("OPTIONS:");
            println!("    -h, --help       Print this help information");
            println!("    -v, --version    Print version information");
            println!();
            println!("KEYBINDINGS:");
            println!("    Space            Play / Pause");
            println!("    z / x, ← / →     Previous / Next track");
            println!("    j / k, ↑ / ↓     Navigate list / tree");
            println!("    Enter            Play track / Expand tree branch");
            println!("    , / .            Seek -5s / +5s");
            println!("    + / = / -        Volume ±5%");
            println!("    m                Mute");
            println!("    s                Shuffle mode");
            println!("    l                Loop mode (None / Track / Queue)");
            println!("    v                Toggle visualizer mode (Spectrum / Waveform)");
            println!("    c                Show / Hide visualizer");
            println!("    F1 - F5          Tabs (Artists, Albums, Tracks, Genres, Explorer)");
            println!("    /                Search");
            println!("    Esc              Cancel search");
            println!("    q, Ctrl+c        Quit");
            return Ok(());
        } else if arg == "-V" || arg == "--version" {
            println!("Apollo 0.1.0");
            return Ok(());
        }
    }

    let mut config = crate::config::load_config();

    fn expand_tilde(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Some(mut home) = dirs::home_dir() {
                home.push(&path[2..]);
                return home;
            }
        }
        PathBuf::from(path)
    }

    let music_paths = if !args.is_empty() {
        let str_args = args.clone();
        config.library_paths = str_args;
        crate::config::save_config(&config);
        args.into_iter().map(|s| expand_tilde(&s)).collect()
    } else if !config.library_paths.is_empty() {
        config.library_paths.clone().into_iter().map(|s| expand_tilde(&s)).collect()
    } else {
        crate::config::AppConfig::default().library_paths.into_iter().map(|s| expand_tilde(&s)).collect()
    };

    let mut app = App::new(music_paths);

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(33); // ~30 FPS

    while !app.should_quit {
        // Handle background events
        app.check_scanner_messages();
        app.check_playback_progress();

        // Draw TUI
        terminal.draw(|f| draw_ui(f, &mut app))?;

        // Handle user input
        if event::poll(tick_rate)? {
            loop {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                    if app.is_searching {
                        match key.code {
                            KeyCode::Esc => {
                                app.is_searching = false;
                                app.search_query.clear();
                            }
                            KeyCode::Enter => {
                                app.is_searching = false;
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.selected_index = 0;
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.selected_index = 0;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                app.should_quit = true;
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.should_quit = true;
                            }
                            KeyCode::Char(' ') => {
                                app.audio.toggle_play_pause();
                            }
                            KeyCode::Char('x') | KeyCode::Right => {
                                app.next_track(true);
                            }
                            KeyCode::Char('z') | KeyCode::Left => {
                                app.previous_track();
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.move_selection_down();
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.move_selection_up();
                            }
                            KeyCode::Enter => {
                                app.handle_enter_key();
                            }
                            KeyCode::Char(',') => {
                                app.audio.seek_relative(-5);
                                app.set_status(app.i18n.t("status_seek_bwd"));
                            }
                            KeyCode::Char('.') => {
                                app.audio.seek_relative(5);
                                app.set_status(app.i18n.t("status_seek_fwd"));
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                app.audio.change_volume(0.05);
                                let vol = (app.audio.volume * 100.0).round() as u32;
                                app.set_status(format!("{}: {}%", app.i18n.t("status_volume"), vol));
                            }
                            KeyCode::Char('-') => {
                                app.audio.change_volume(-0.05);
                                let vol = (app.audio.volume * 100.0).round() as u32;
                                app.set_status(format!("{}: {}%", app.i18n.t("status_volume"), vol));
                            }
                            KeyCode::Char('m') => {
                                app.audio.toggle_mute();
                                if app.audio.is_muted {
                                    app.set_status(app.i18n.t("status_muted"));
                                } else {
                                    let vol = (app.audio.volume * 100.0).round() as u32;
                                    app.set_status(format!("{}: {}%", app.i18n.t("status_unmuted"), vol));
                                }
                            }
                            KeyCode::Char('s') => {
                                app.audio.toggle_shuffle();
                                app.set_status(if app.audio.shuffle {
                                    app.i18n.t("status_shuffle_on").to_string()
                                } else {
                                    app.i18n.t("status_shuffle_off").to_string()
                                });
                            }
                            KeyCode::Char('l') => {
                                app.audio.cycle_loop_mode();
                                let mode_str = match app.audio.loop_mode {
                                    crate::audio::LoopMode::Off => app.i18n.t("loop_none"),
                                    crate::audio::LoopMode::Track => app.i18n.t("loop_track"),
                                    crate::audio::LoopMode::Queue => app.i18n.t("loop_queue"),
                                };
                                app.set_status(format!("{}: {}", app.i18n.t("status_loop"), mode_str));
                            }
                            KeyCode::Char('v') => {
                                app.visualizer.toggle_mode();
                                app.set_status(format!("{}: {:?}", app.i18n.t("status_visualizer"), app.visualizer.mode));
                            }
                            KeyCode::Char('c') => {
                                app.show_visualizer = !app.show_visualizer;
                                app.set_status(if app.show_visualizer {
                                    app.i18n.t("status_visualizer_visible")
                                } else {
                                    app.i18n.t("status_visualizer_hidden")
                                });
                            }
                            KeyCode::F(1) => {
                                app.switch_tab(LibraryTab::Artists);
                            }
                            KeyCode::F(2) => {
                                app.switch_tab(LibraryTab::Albums);
                            }
                            KeyCode::F(3) => {
                                app.switch_tab(LibraryTab::Tracks);
                            }
                            KeyCode::F(4) => {
                                app.switch_tab(LibraryTab::Genres);
                            }
                            KeyCode::F(5) => {
                                app.switch_tab(LibraryTab::Explorer);
                            }
                            KeyCode::Char('/') => {
                                app.is_searching = true;
                                app.search_query.clear();
                            }
                            _ => {}
                        }
                    }
                        }
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            crossterm::event::MouseEventKind::ScrollDown => {
                                app.move_selection_down();
                            }
                            crossterm::event::MouseEventKind::ScrollUp => {
                                app.move_selection_up();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
            }
        }
    }

    // Gracefully stop background threads and cleanup
    app.save_state();
    app.cancel_flag.store(true, Ordering::Relaxed);
    app.audio.stop();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
