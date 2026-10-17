# Changelog

All notable changes to this project will be documented in this file.

## [1.2.0] - 2026-10-17

### Added
- **Text Wrapping in Library**: Long artist, album, track, genre and file names now wrap onto additional lines instead of being clipped. Applies to all tabs (Artists, Albums, Tracks, Genres, Explorer).
- **Metadata Below Title**: When a track row doesn't fit on one line, its file metadata (duration, format) is rendered whole on a new line below the song name. In the Tracks tab on narrow panels (< 80 columns), the Time/Format columns are folded under the title.
- **Wrapping Status Info**: The loop / shuffle / queue / format info and `[i]` alerts in the "Now Playing" panel now wrap at the `|` separators, and the panel grows in height to fit them.

### Changed
- **Responsive Library Tabs**: The `F1`-`F5` tab labels in the library title bar are hidden when the panel is too narrow to fit them, instead of clipping. Shortcuts keep working.
- **Compact Now Playing Labels**: On narrow panels the "Playing:" / "Volume:" text labels are dropped, leaving only the state icon (e.g. `[>]`) and the volume bar.
- **Full-Width Selection Highlight**: The selected row highlight now spans the full width of the library panel, including all lines of wrapped items.

### Fixed
- **Alerts Clipping**: Status alerts in the "Now Playing" panel no longer get cut off when the terminal is narrow.
- **Scrolling with Multi-Line Rows**: Scrolling now correctly keeps the selected item visible when rows span multiple lines.

## [1.1.0] - 2026-10-04

### Added
- **Configuration Options**: Added new fields to `config.toml`:
  - `visualizer_color`: Choose between `"solid"` (terminal theme color) or `"rainbow"` (multi-color).
  - `show_logo`: Toggle the Apollo logo visibility. If `false`, the library panel dynamically expands to claim the freed space.
- **Persistent Memory**: The application state (selected tab, volume, visualizer visibility, visualizer mode, hotkeys visibility) is now persistently saved across terminal sessions.
- **Hotkey Toggle (H)**: Added the `H` shortcut to hide all hotkey hints from the footer and the visualizer title. This shrinks the footer and dynamically scales the UI for a cleaner look.

### Changed
- **Modern UI Shortcuts**: Replaced the classic `[ x ]` bracketed shortcuts with modern, solid terminal blocks (cyan background, black text) across the footer and active tabs.
- **Tab Headers Relocation**: Moved library tabs (`F1`-`F5`) seamlessly into the library's top border title line (like the visualizer), freeing up vertical space for tracks.
- **Layout Margins**: Added symmetric top, bottom, left, and right padding to the library and "Now playing" panels. Cursors and text no longer stick directly to the borders.

### Fixed
- **Terminal Theme Adaptability (Light/Dark)**: Purged hardcoded white colors and forced explicit contrasts (e.g., black text on cyan blocks, and `Color::Reset` for standard text). The UI is now completely readable regardless of the terminal's theme scheme.
- **UI Scaling Glitch on Alerts**: Moved status alerts (`[i]`) from the footer to the "Now Playing" module (next to the Format tag). Status popups no longer cause the whole interface grid to unpredictably bounce and scale.

