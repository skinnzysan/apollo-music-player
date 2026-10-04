# Changelog

All notable changes to this project will be documented in this file.

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

