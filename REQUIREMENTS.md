# Specyfikacja Wymagań Projektu: CLI Music Player dla Linuksa

Dokument definiuje architekturę, wymagania funkcjonalne, niefunkcjonalne oraz projekt interfejsu użytkownika dla terminalowego odtwarzacza muzyki (TUI) dedykowanego systemom z rodziny Linux.

---

## 1. Cel Projektu i Założenia Ogólne

Głównym celem jest stworzenie nowoczesnego, ultralekkiego i responsywnego odtwarzacza muzycznego działającego bezpośrednio w terminalu (CLI/TUI). Aplikacja ma charakteryzować się minimalnym zużyciem zasobów procesora i pamięci RAM, natychmiastowym czasem startu, wsparciem dla szerokiej gamy formatów audio oraz estetycznym interfejsem pełnoekranowym z podziałem modułowym i wbudowanym wizualizatorem częstotliwości.

---

## 2. Proponowany Stack Technologiczny

Do realizacji zadania wybrano język **Rust** ze względu na bezkonkurencyjną wydajność, bezpieczeństwo pamięci, brak narzutu Garbage Collectora oraz dojrzały ekosystem bibliotek multimedialnych i terminalowych.

| Obszar | Technologia / Biblioteka | Uzasadnienie |
| :--- | :--- | :--- |
| **Język programowania** | **Rust** (edycja 2021+) | Maksymalna wydajność, praca wielowątkowa bez wyścigów (*data races*), zerowy koszt abstrakcji. |
| **Interfejs TUI** | **`ratatui`** + **`crossterm`** | Wiodący framework TUI w Ruście. Oferuje deklaratywne definiowanie widoków, wsparcie dla ramek ASCII/Unicode, automatyczne skalowanie przy zmianie rozmiaru okna oraz obsługę zdarzeń klawiatury i myszy. |
| **Silnik i dekodowanie audio** | **`symphonia`** | Czysty, bezpieczny dekoder w Ruście obsługujący bezstratne i stratne formaty: FLAC, MP3, WAV, OGG/Vorbis, Opus, AAC, ALAC. |
| **Wyjście audio (Backend)** | **`cpal`** / **`rodio`** | Niskopoziomowa, wieloplatformowa integracja z podsystemami audio Linuksa: **PipeWire**, **PulseAudio** oraz **ALSA**. |
| **Metadane utworów** | **`lofty`** | Ekstremalnie szybki odczyt i parsowanie tagów ID3v1/v2, Vorbis Comments, FLAC tags, MP4 ilst. |
| **Wizualizator (DSP/FFT)** | **`rustfft`** | Błyskawiczna realizacja szybkiej transformaty Fouriera (FFT) na buforach PCM pobieranych z silnika odtwarzania. |
| **Baza danych / Cache** | **`rusqlite`** lub binarny cache w pamięci | Błyskawiczne ładowanie i indeksowanie biblioteki `~/Music` bez konieczności ponownego parsowania wszystkich plików przy każdym uruchomieniu. |

---

## 3. Architektura Systemu

Aplikacja opiera się na architekturze wielowątkowej z asynchroniczną komunikacją za pomocą kanałów (*mpsc* / *crossbeam*):

```
+-----------------------------------------------------------------------+
|                             Główny Wątek (TUI)                        |
|  - Pętla zdarzeń (Event Loop: 30-60 FPS)                              |
|  - Obsługa wejścia (Klawiatura / Mysz / SIGWINCH - Resize)            |
|  - Renderowanie widoków Ratatui w oparciu o stan aplikacji            |
+-------------------^-------------------------------^-------------------+
                    |                               |
       Zdarzenia UI / Akcje            Dane widma FFT / Status
                    |                               |
+-------------------v-------------------+  +--------+-------------------+
|       Wątek Audio (Playback)          |  |     Wątek Wizualizatora    |
|  - Dekodowanie przez Symphonia        |  |  - Bufor kołowy próbek PCM |
|  - Strumieniowanie próbek do CPAL     |  |  - Obliczanie FFT          |
|  - Pauza / Wznawianie / Seek          |  |  - Skalowanie na pasma     |
+---------------------------------------+  +----------------------------+
                    |
+-------------------v-------------------+
|      Wątek Skanera Biblioteki         |
|  - Przeszukiwanie ~/Music w tle       |
|  - Wyciąganie metadanych (Lofty)      |
|  - Aktualizacja indeksu / bazy SQLite |
+---------------------------------------+
```

---

## 4. Wymagania Funkcjonalne

### 4.1. Obsługa Formatów Dźwiękowych
- **Formaty bezstratne:** FLAC, WAV, ALAC.
- **Formaty stratne:** MP3, OGG Vorbis, Opus, AAC (M4A).
- Obsługa plików o zmiennym i stałym bitrate (CBR/VBR) oraz różnych częstotliwościach próbkowania (44.1 kHz, 48 kHz, 96 kHz, Hi-Res).

### 4.2. Sterowanie Odtwarzaniem (Playback Control)
- **Start / Wznów / Wstrzymaj (Play / Pause / Resume)** – natychmiastowa reakcja bez słyszalnych klików (*fade-in / fade-out* 10-20 ms).
- **Następny / Poprzedni utwór (Next / Previous)**.
- **Przewijanie (Seeking):** Skok w przód / w tył o konfigurowalny interwał (np. 5s / 10s).
- **Tryby odtwarzania:**
  - Standardowy (kolejno z listy),
  - Pętla (Loop): wyłączona / zapętlenie pojedynczego utworu / zapętlenie całej listy,
  - Mieszanie (Shuffle): pseudolosowe tasowanie kolejki z zachowaniem historii odtworzeń.
- **Regulacja głośności:** Płynna regulacja (krok 5%) z funkcją wyciszenia (Mute).

### 4.3. Zarządzanie Biblioteką Muzyczną
- **Źródło plików:** Automatyczne skanowanie katalogu domowego użytkownika: `~/Music` (rekurencyjnie).
- **Metadane:** Odczyt tagów ID3/Vorbis/FLAC:
  - Tytuł utworu,
  - Wykonawca (Artist) oraz Wykonawca albumu (Album Artist),
  - Tytuł albumu,
  - Gatunek muzyczny (Genre),
  - Numer utworu / płyty (Track/Disc number),
  - Rok wydania,
  - Czas trwania pliku.
- **Grupy i widoki biblioteki:**
  - **Wg Wykonawców (Artists):** Wykonawca -> Albumy -> Utwory.
  - **Wg Albumów (Albums):** Alfabetyczna lista albumów z rokiem i wykonawcą.
  - **Wg Gatunków (Genres):** Gatunek -> Utwory / Albumy.
  - **Wszystkie utwory (Tracks):** Płaska lista z sortowaniem po dacie dodania, nazwie lub wykonawcy.
  - **Eksplorator plików:** Bezpośredni widok struktury katalogów na wypadek braku tagów.
- **Wyszukiwarka i filtracja:**
  - Interaktywne wyszukiwanie typu *fuzzy search* (filtrowanie na żywo podczas wpisywania).

### 4.4. Wizualizator Audio
- **Analizator pasma częstotliwości (Spectrum Analyzer):**
  - Dzielenie pasma akustycznego (20 Hz - 20 kHz) na słupki za pomocą logarytmicznej skali częstotliwości.
  - Renderowanie słupków za pomocą bloków Unicode (` `, `▂`, `▃`, `▄`, `▅`, `▆`, `▇`, `█`) lub czystych znaków ASCII (np. `|`, `#`, `=`) w zależności od konfiguracji terminala.
  - Efekt opadania słupków (*gravity / peak decay*) zapewniający płynną animację.
- **Opcjonalny tryb fali (Oscilloscope / Waveform):** Rysowanie amplitudy fali dźwiękowej w czasie rzeczywistym.

---

## 5. Projekt Interfejsu Użytkownika (TUI & Układ)

### 5.1. Założenia Wizualne i Ramy ASCII
- Aplikacja zajmuje 100% szerokości i wysokości terminala (`Alternate Screen`).
- Dynamiczne przeliczanie wymiarów przy zmianie rozmiaru okna terminala (obsługa sygnału `SIGWINCH`).
- Wyraźny podział na 3 główne panele za pomocą ramek ASCII / Unicode Box-Drawing:
  1. **Panel Główny (Lista utworów / Biblioteka)** – lewa / górna sekcja.
  2. **Panel Wizualizatora** – dolna lub boczna sekcja.
  3. **Panel Kontroli i Stanu (Now Playing)** – pasek statusu i informacji o aktualnym utworze.

### 5.2. Kolorystyka i Paleta Barw Aktualnego Terminala
- **Wykorzystanie palety terminala:** Interfejs nie narzuca sztywnych barw HEX/TrueColor, lecz w całości adaptuje się do schematu kolorów aktualnie używanego emulatora terminala.
- **Wsparcie dla 16 standardowych kolorów ANSI:** Wszystkie elementy wizualne bazują na standardowych kolorach ANSI (Black, Red, Green, Yellow, Blue, Magenta, Cyan, White oraz ich wariantach Bright), dzięki czemu aplikacja natywnie wygląda spójnie z motywami użytkownika (np. Gruvbox, Catppuccin, Nord, Tokyo Night, Dracula, Solarized).
- **Przezroczyste / Domyślne Tło:** Wykorzystanie koloru tła terminala (`Color::Reset` w Ratatui), co pozwala na zachowanie ewentualnej przezroczystości terminala (*transparency / blur*) skonfigurowanej w systemie.
- **Semantyczne mapowanie kolorów:**
  - Aktywny panel / zaznaczony utwór: Kolor akcentu terminala (np. Cyan / Blue).
  - Wskaźniki odtwarzania (Play/Pause, Pętla, Mieszanie): ANSI Green (odtwarzanie), ANSI Yellow (pauza/ostrzeżenia).
  - Wizualizator: Słupki rysowane z użyciem kolorów ANSI (np. gradient od Cyan przez Green do Red przy szczytowych wartościach).
  - Metadane i ramki nieaktywne: ANSI Bright Black / Muted Gray.

### 5.3. ASCII Mockup Interfejsu

Poniższy schemat przedstawia domyślny układ w standardowym oknie terminala (np. 100x28 znaków):

```
+--[ 1. BIBLIOTEKA MUZYCZNA ]---------------------------------------[ Zakładki: [F1] Wykonawcy | [F2] Albumy | [F3] Wszystkie ]--+
| Wykonawca / Album                    | Tytuł                              | Czas  | Gatunek     | Format                     |
+--------------------------------------+------------------------------------+-------+-------------+----------------------------+
| > Pink Floyd                         |                                    |       |             |                            |
|   v The Dark Side of the Moon (1973) |                                    |       |             |                            |
|       01. Speak to Me                | Speak to Me                        | 01:08 | Prog Rock   | FLAC 24bit/96kHz           |
|     * 02. Breathe                    | Breathe (In the Air)               | 02:49 | Prog Rock   | FLAC 24bit/96kHz           |
|       03. On the Run                 | On the Run                         | 03:45 | Prog Rock   | FLAC 24bit/96kHz           |
|       04. Time                       | Time                               | 06:53 | Prog Rock   | FLAC 24bit/96kHz           |
|   > Wish You Were Here (1975)        |                                    |       |             |                            |
| > Daft Punk                          |                                    |       |             |                            |
| > Miles Davis                        |                                    |       |             |                            |
|                                      |                                    |       |             |                            |
+--[ 2. WIZUALIZATOR AUDIO (SPECTRUM) ]----------------------------------------------------------------------------------------+
|                                                                                                                              |
|            █                                                                                                                 |
|          █ █ █        █                                                                                                      |
|        █ █ █ █ █    █ █ ▄                                                                                                    |
|      █ █ █ █ █ █ █  █ █ █ █ ▄                                                                                                |
|    █ █ █ █ █ █ █ █  █ █ █ █ █ █ ▄      ▄                                                                                     |
|  █ █ █ █ █ █ █ █ █  █ █ █ █ █ █ █ █  ▄ █ ▄ ▄                                                                                  |
+--[ 3. TERAZ ODTWARZANE ]----------------------------------------------------------------------------------------------------+
|  [>] Odtwarzanie: Pink Floyd - Breathe (In the Air)                                            Głośność: [████████░░] 80%    |
|  01:14 [============================--------------------------------------------------------] 02:49                         |
|  Status: [Zapętlenie: Utwór] | [Mieszanie: WŁ] | Kolejka: 2/10 | Format: FLAC 96kHz 24-bit                                   |
+--[ KLAWISZOLOGIA: [Spacja] Pauza/Play | [n]/[p] Nast/Poprz | [s] Shuffle | [r] Pętla | [/] Szukaj | [q] Wyjście ]-----------------+
```

---

## 6. Domyślne Skróty Klawiszowe

| Klawisz | Akcja |
| :--- | :--- |
| `Spacja` | Wstrzymaj / Wznów odtwarzanie (Play / Pause) |
| `n` / `p` | Następny / Poprzedni utwór (Next / Previous) |
| `j` / `k` lub `Strzałka w dół` / `w górę` | Nawigacja po liście utworów |
| `Enter` | Odtwórz zaznaczony utwór / Rozwiń gałąź drzewa |
| `h` / `l` lub `Strzałka w lewo` / `w prawo` | Przewijanie utworu o -5s / +5s |
| `+` / `-` lub `]` / `[` | Zwiększ / Zmniejsz głośność o 5% |
| `m` | Wycisz / Przywróć dźwięk (Mute) |
| `s` | Włącz / Wyłącz tryb losowy (Shuffle) |
| `r` | Przełącz tryb pętli (Brak -> Utwór -> Kolejka) |
| `Tab` | Przełączenie aktywnego panelu (Fokus) |
| `F1` - `F4` | Zmiana widoku biblioteki (Wykonawcy, Albumy, Gatunki, Wszystkie) |
| `/` | Wyszukiwanie (otwarcie paska wpisywania filtru) |
| `Esc` | Wyjście z trybu szukania / Powrót |
| `q` | Zamknięcie aplikacji |

---

## 7. Wymagania Niefunkcjonalne

1. **Wydajność i Zużycie Zasobów:**
   - Obciążenie CPU w trakcie odtwarzania z włączonym wizualizatorem: **< 2%** na przeciętnym procesorze 4-rdzeniowym.
   - Zużycie pamięci RAM: **< 30 MB** przy bibliotece liczącej do 10 000 utworów.
   - Czas uruchomienia (Cold Start): **< 100 ms**.
2. **Responsywność i Skalowanie:**
   - Brak blokowania wątku interfejsu (UI) podczas dekodowania i operacji I/O na dysku.
   - Płynne skalowanie układu przy zmianie rozmiaru okna terminala (minimalny obsługiwany rozmiar: 60x18 znaków, z ukrywaniem wizualizatora przy skrajnie małych oknach).
3. **Stabilność:**
   - Odporność na uszkodzone lub niepełne pliki audio (aplikacja powinna pominąć uszkodzony plik i wyświetlić ostrzeżenie w pasku statusu zamiast kończyć działanie błędem *panic*).
4. **Zgodność z systemem Linux:**
   - Działanie na czystej konsoli TTY, emulatorach terminali (Alacritty, Kitty, Foot, WezTerm, GNOME Terminal) oraz sesjach `tmux` / `screen`.

---

## 8. Proponowany Harmonogram Implementacji

- [ ] **Krok 1: Inicjalizacja projektu i silnik audio:**
  - Konfiguracja `Cargo.toml` z zależnościami (`symphonia`, `cpal`, `rodio`).
  - Implementacja podstawowego wątku odtwarzacza (dekodowanie i wyjście audio, play/pause/seek).
- [ ] **Krok 2: Indeksowanie biblioteki i metadane:**
  - Integracja biblioteki `lofty`.
  - Rekurencyjny skaner `~/Music` działający w tle.
  - Struktury danych do grupowania (Artysta, Album, Gatunek).
- [ ] **Krok 3: Interfejs TUI (Ratatui):**
  - Implementacja układu blokowego z ramkami ASCII.
  - Wyświetlanie listy utworów, obsługa zaznaczania i skrótów klawiszowych.
  - Panel "Now Playing" z paskiem postępu.
- [ ] **Krok 4: Wizualizator audio:**
  - Pobieranie próbek PCM z bufora odtwarzacza.
  - Obliczenia FFT za pomocą `rustfft` i renderowanie słupków częstotliwości.
- [ ] **Krok 5: Skalowanie, dopracowanie i testy wydajnościowe:**
  - Obsługa małych rozmiarów okna.
  - Profilowanie zużycia CPU i pamięci RAM.
