# Apollo - Terminalowy Odtwarzacz Muzyki dla Linuksa (TUI)

Apollo to nowoczesny, lekki i responsywny terminalowy odtwarzacz muzyczny (TUI) dla systemu Linux, napisany w języku **Rust** w oparciu o biblioteki **Ratatui**, **Crossterm**, **Rodio / Symphonia**, **Lofty** oraz **RustFFT**.

## Główne Cechy

- **Ultraszybki start i oszczędność zasobów:** Cold start < 100 ms, obciążenie CPU < 2%, zużycie pamięci RAM < 30 MB.
- **Szeroka obsługa formatów audio:** FLAC, WAV, MP3, OGG Vorbis, Opus, AAC (M4A), ALAC (dzięki czysto rustowym dekoderom `symphonia`).
- **Natywna integracja z systemem dźwięku Linux:** PipeWire, PulseAudio oraz ALSA.
- **Wizualizator audio w czasie rzeczywistym:**
  - Analizator widma FFT (20 Hz - 20 kHz) w skali logarytmicznej z gładką grawitacją opadania słupków.
  - Wykorzystanie bloków Unicode (` `, `▂`, `▃`, `▄`, `▅`, `▆`, `▇`, `█`) z 8 poziomami sub-pikseli.
  - Opcjonalny tryb oscyloskopu / fali PCM (przełączany klawiszem `v`).
- **Zarządzanie biblioteką muzyczną:**
  - Automatyczne, asynchroniczne skanowanie `~/Music` w tle bez blokowania interfejsu.
  - Szybki cache indeksu metadanych w `~/.cache/apollo/library_cache.json`.
  - Widoki i zakładki:
    - `[F1]` **Wykonawcy:** hierarchiczne drzewo Wykonawca -> Albumy -> Utwory.
    - `[F2]` **Albumy:** lista albumów z rokiem wydania i liczbą utworów.
    - `[F3]` **Wszystkie:** płaska tabela utworów.
    - `[F4]` **Gatunki:** grupowanie utworów wg gatunku muzycznego.
    - `[F5]` **Eksplorator:** bezpośrednie przeglądanie plików w strukturze katalogów.
  - Wyszukiwanie na żywo (*live search* / *fuzzy filter*) pod klawiszem `/`.
- **Dopasowanie do terminala użytkownika:**
  - Adaptacja do 16 kolorów ANSI bieżącego schematu barw (Gruvbox, Catppuccin, Nord, Tokyo Night, Dracula itd.).
  - Przezroczyste tło (`Color::Reset`) zachowujące przezroczystość i rozmycie okna.
  - Responsywne skalowanie z dynamicznym ukrywaniem wizualizatora w małych oknach.

---

## Skróty Klawiszowe

| Klawisz | Akcja |
| :--- | :--- |
| `Spacja` | Wstrzymaj / Wznów odtwarzanie (Play / Pause) |
| `n` / `p` | Następny / Poprzedni utwór (Next / Previous) |
| `j` / `k` lub `↓` / `↑` | Nawigacja po liście utworów / drzewie |
| `Enter` | Odtwórz utwór / Rozwiń/zwiń gałąź drzewa |
| `h` / `l` lub `←` / `→` | Przewijanie o -5s / +5s |
| `+` / `-` lub `]` / `[` | Zwiększ / Zmniejsz głośność o 5% |
| `m` | Wycisz / Przywróć dźwięk (Mute) |
| `s` | Włącz / Wyłącz tryb losowy (Shuffle) |
| `r` | Przełącz tryb pętli (Brak -> Utwór -> Kolejka) |
| `v` | Przełącz tryb wizualizatora (Spectrum FFT / Fala PCM) |
| `Tab` | Przełącz aktywny panel |
| `F1` - `F5` | Zmiana zakładki biblioteki |
| `/` | Wyszukiwanie (filtr na żywo) |
| `Esc` | Anulowanie wyszukiwania / powrót |
| `q` | Wyjście z programu |

---

## Uruchomienie i Budowanie

### Wymagania systemowe (Linux)
- Zainstalowany kompilator Rusta (`cargo`, `rustc`).
- Biblioteka ALSA (`libasound2-dev` na Debianie/Ubuntu lub `alsa-lib` na Arch/CachyOS).

### Uruchomienie deweloperskie:
```bash
cargo run
```

### Uruchomienie wersji zoptymalizowanej (Release):
```bash
cargo run --release
```

Możesz również podać własny katalog z muzyką jako argument:
```bash
./target/release/apollo /ścieżka/do/muzyki
```
