pub mod app;
pub mod audio;
pub mod library;
pub mod ui;
pub mod visualizer;

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
            println!("Apollo - Terminalowy Odtwarzacz Muzyki dla Linuksa");
            println!();
            println!("UŻYCIE:");
            println!("    apollo [KATALOG_LUB_PLIK...]");
            println!();
            println!("OPCJE:");
            println!("    -h, --help       Wyświetl tę pomoc");
            println!("    -v, --version    Wyświetl wersję programu");
            println!();
            println!("SKRÓTY KLAWISZOWE:");
            println!("    Spacja           Pauza / Wznów (Play/Pause)");
            println!("    n / p            Następny / Poprzedni utwór");
            println!("    j / k, ↑ / ↓     Poruszanie się po liście");
            println!("    Enter            Odtwórz utwór / Rozwiń gałąź");
            println!("    h / l, ← / →     Przewijanie -5s / +5s");
            println!("    + / -, ] / [     Głośność ±5%");
            println!("    m                Wyciszenie (Mute)");
            println!("    s                Mieszanie losowe (Shuffle)");
            println!("    r                Zapętlenie (Brak / Utwór / Kolejka)");
            println!("    v                Tryb wizualizatora (Spectrum / Waveform)");
            println!("    F1 - F5          Zakładki (Wykonawcy, Albumy, Wszystkie, Gatunki, Eksplorator)");
            println!("    Tab              Przełącz aktywny panel");
            println!("    /                Wyszukiwanie");
            println!("    q, Esc           Wyjście / Anulowanie");
            return Ok(());
        } else if arg == "-V" || arg == "--version" {
            println!("Apollo 0.1.0");
            return Ok(());
        }
    }

    let music_paths = if !args.is_empty() {
        args.into_iter().map(PathBuf::from).collect()
    } else {
        let mut paths = Vec::new();
        if let Some(music_dir) = dirs::audio_dir() {
            paths.push(music_dir);
        } else if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Music"));
        }
        paths.push(PathBuf::from("."));
        paths
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
            if let Event::Key(key) = event::read()? {
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
                            KeyCode::Char('n') => {
                                app.next_track(true);
                            }
                            KeyCode::Char('p') => {
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
                            KeyCode::Char('h') | KeyCode::Left => {
                                app.audio.seek_relative(-5);
                                app.set_status("Przewinięto: -5s");
                            }
                            KeyCode::Char('l') | KeyCode::Right => {
                                app.audio.seek_relative(5);
                                app.set_status("Przewinięto: +5s");
                            }
                            KeyCode::Char('+') | KeyCode::Char(']') | KeyCode::Char('=') => {
                                app.audio.change_volume(0.05);
                                let vol = (app.audio.volume * 100.0).round() as u32;
                                app.set_status(format!("Głośność: {}%", vol));
                            }
                            KeyCode::Char('-') | KeyCode::Char('[') => {
                                app.audio.change_volume(-0.05);
                                let vol = (app.audio.volume * 100.0).round() as u32;
                                app.set_status(format!("Głośność: {}%", vol));
                            }
                            KeyCode::Char('m') => {
                                app.audio.toggle_mute();
                                if app.audio.is_muted {
                                    app.set_status("Dźwięk wyciszony");
                                } else {
                                    let vol = (app.audio.volume * 100.0).round() as u32;
                                    app.set_status(format!("Przywrócono dźwięk: {}%", vol));
                                }
                            }
                            KeyCode::Char('s') => {
                                app.audio.toggle_shuffle();
                                app.set_status(format!(
                                    "Tryb losowy: {}",
                                    if app.audio.shuffle { "WŁĄCZONY" } else { "WYŁĄCZONY" }
                                ));
                            }
                            KeyCode::Char('r') => {
                                app.audio.cycle_loop_mode();
                                app.set_status(format!(
                                    "Tryb zapętlania: {}",
                                    app.audio.loop_mode.display_label()
                                ));
                            }
                            KeyCode::Char('v') => {
                                app.visualizer.toggle_mode();
                                app.set_status(format!(
                                    "Tryb wizualizatora: {:?}",
                                    app.visualizer.mode
                                ));
                            }
                            KeyCode::Tab => {
                                app.toggle_focus();
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
            } else if let Event::Mouse(mouse) = event::read()? {
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
        }
    }

    // Gracefully stop background threads and cleanup
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
