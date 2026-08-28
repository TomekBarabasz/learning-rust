# Handoff — Nope, Finish This First!

Dokument do wklejenia na początku nowego czatu. Zawiera decyzje projektowe
i pułapki, które kosztowały czas — bez nich asystent powtórzy te same błędy.

---

## Czym jest aplikacja

Desktopowy licznik czasu pracy w Ruście, GUI na eframe/egui, jedno okno.
Wymusza dokończenie zadeklarowanego minimum, pozwala pracować dłużej,
loguje sesje do CSV. Kompiluje się na Windows i Linux, docelowa platforma
to Windows.

## Stany i przepływ

```
idle ──[nowe zadanie → dialog]──▶ busy ──[minimum wypełnione]──▶ overtime
 ▲                                  │  ▲                            │
 │                                  ▼  │                            │
 └──────────[zakończ]───────────  pauza ┘        [przedłuż +15/30/45]┘
                                                    (wraca do busy)
```

- **idle** — animacja `idle.gif`, przycisk „nowe zadanie", wybór pliku logu
- **busy** — `busy.gif`, cztery wiersze: zadanie / Koniec / Czas sesji / Pozostało
- **overtime** — `overtime.gif`, trzy wiersze: zadanie / Minimum od / Czas sesji (zielony)
- **pauza** — `pause.gif`, licznik zamrożony, dostępna w obu stanach pracy

Aplikacja **nigdy sama nie wraca do idle**. Jedyne wyjście to przycisk „zakończ",
który zapisuje wiersz do CSV. Po wypełnieniu minimum leci jednorazowe mrugnięcie
w pasku zadań (flaga `notified`).

W stanie pracy okno jest ustawiane jako always-on-top i ma wyłączony przycisk
minimalizacji; przy powrocie do idle oba ustawienia są cofane.

## Model czasu — sedno logiki

`Task` mierzy **przepracowany czas**, nie „kiedy koniec":

```rust
started: Instant,              // start sesji
started_at: DateTime<Local>,   // to samo wg zegara ściennego, do logu
minimum: Duration,             // aktualne zobowiązanie
paused_total: Duration,        // suma zakończonych przerw
paused_at: Option<Instant>,    // początek trwającej przerwy
```

- `worked()` = czas od startu minus wszystkie przerwy (także trwająca)
- `remaining()` = `minimum - worked()`, zero znaczy „minimum wypełnione"
- `is_overtime()` = `worked() >= minimum`
- `extend(extra)` ustawia **`minimum = worked() + extra`**, nie `minimum += extra`

Ta ostatnia rzecz jest nieoczywista i celowa: po przedłużeniu „Pozostało" ma
pokazywać dokładnie tyle, ile użytkownik przed chwilą zakomitował, a nie
pomniejszone o dotychczasową nadwyżkę. Czas sesji leci dalej bez resetu.

`Instant` (monotoniczny) do liczenia, `chrono::Local` osobno do wyświetlania
i logu — zmiana czasu systemowego nie psuje pomiaru.

## Odświeżanie

Interfejs budzi się dokładnie w chwili zmiany wyświetlanej minuty, przez
`ctx.request_repaint_after(...)`. W stanie busy tykają dwa liczniki naraz
(czas sesji rośnie, zaokrąglany w dół; „Pozostało" maleje, zaokrąglane w górę),
więc brany jest wcześniejszy z dwóch terminów. Podczas pauzy nie planujemy
przebudzeń w ogóle.

---

## Pułapki API — eframe/egui 0.36

**To jest najważniejsza sekcja.** Wersja 0.36 jest nowsza niż wiedza modeli;
poniższe rzeczy trzeba znać, inaczej kod się nie skompiluje.

| Rzecz | Stan w 0.36 |
|---|---|
| Trait `eframe::App` | wymaga `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)` — **nie** `update` |
| `Context` w `ui` | przez `ui.ctx().clone()` (tani uchwyt Arc) |
| `CentralPanel::show` | przyjmuje `&mut Ui`, nie `&Context` |
| `Window::show` | nadal `&Context` |
| `ComboBox` | `from_id_salt`, nie `from_id_source` |
| `Image::corner_radius` | niesprawdzone, unikane (w 0.31 `Rounding` → `CornerRadius`) |

Zasada robocza: **przy każdym nowym elemencie egui zweryfikować API w sieci**,
zamiast pisać z pamięci. Ten projekt stracił na tym dwa cykle kompilacji.

## Ładowanie animacji

Loader `file://` był źródłem problemów, więc jest **omijany**. Pliki są czytane
`std::fs::read` przy starcie i wrzucane do cache egui:

```rust
ctx.include_bytes(format!("bytes://{stem}.{ext}"), bytes);
egui::Image::new(uri.as_str())
```

Rozszerzenie w URI musi zostać — po nim egui rozpoznaje format i włącza loader
animowany. Kolejność szukania: plik na dysku (`assets/` obok exe, potem
w katalogu roboczym, rozszerzenia `.webp` → `.gif` → `.png`), a jeśli nie ma —
wersja wkompilowana przez `include_bytes!` (tablica `EMBEDDED`). Dzięki temu exe
jest samowystarczalny, ale animację można podmienić bez rekompilacji.

Wymagane feature'y: `egui_extras = ["image", "gif", "webp"]` **oraz**
`image = ["gif", "webp", "png"]`. Włączenie tylko w jednym miejscu nie działa.

Animowany WebP bywa kapryśny — GIF-y są sprawdzone i działają.

## Log CSV

Wybierany przez natywny dialog (`rfd`), ścieżka żyje do zmiany albo zamknięcia
aplikacji (nie jest zapisywana między uruchomieniami). Bez wybranego pliku
logowanie jest nieaktywne. Zapis w trybie append, nagłówek tylko dla pustego pliku.

```
started_at,ended_at,task,tag,minimum_min,worked_min,paused_min
2026-08-17 09:00:00,2026-08-17 10:35:00,"Raport, wersja ""2""",papiery,60,95,7
```

Pola escapowane wg RFC 4180 (cudzysłowy podwajane), czasy zaokrąglane do minut,
UTF-8 bez BOM. `minimum_min` to zobowiązanie **po** wszystkich przedłużeniach.
Zapis następuje **przed** zmianą stanu na idle — potem zadania już nie ma.

W repo jest `analiza.py` z przykładami grupowania w pandas.

## Logowanie diagnostyczne

`env_logger`, poziom sterowany `RUST_LOG`, domyślnie `info`. W debug na stderr,
w release do pliku `nope.log` obok exe — bo `windows_subsystem = "windows"`
odcina konsolę i stderr trafia w próżnię. `init()` wołane raz, na końcu
konfiguracji buildera.

## Budowanie i ikony

- `build.rs` **w katalogu pakietu, nie w `src/`** — częsty błąd, cargo po cichu
  ignoruje plik w złym miejscu i exe wychodzi bez ikony
- `assets/icon.ico` → ikona exe przez `winresource` (wymaga `rc.exe` z Windows SDK)
- `assets/icon.png` → ikona okna przez `ViewportBuilder::with_icon` + crate `image`
- `.ico` powinien mieć komplet rozmiarów (16/32/48/256); Eksplorator cache'uje
  ikony, `ie4uinit.exe -show` odświeża
- feature `kiosk` (opcjonalny) — wymusza przywracanie okna po Win+D

## Uruchamianie na maszynie firmowej

AppLocker blokuje uruchamianie z katalogów zapisywalnych przez użytkownika,
więc `target\release\*.exe` nie startuje („Access is denied"). Działa
`%LOCALAPPDATA%\Programs\` — tam lądują instalacje user-scope i ten katalog
jest na białej liście. Deployment to skopiowanie jednego exe (animacje i ikony
są wkompilowane).

---

## Konwencje w kodzie

- Komentarze i interfejs po polsku, nazwy symboli po angielsku
- Pliki mają zakończenia linii **CRLF** (edycja z Windows)
- Stałe konfiguracyjne na górze pliku: `ANIM_SIZE`, `TEXT_SIZE`, `BUTTON_H`,
  `EXTEND_OPTIONS`, `OVERTIME_COLOR`, `APP_NAME`
- Wysokość elementów w wierszu przycisków bierze się z `BUTTON_H`; ComboBox nie
  przyjmuje `add_sized`, więc dostaje wysokość przez `ui.spacing_mut()`
- Odstęp wyśrodkowujący prawą kolumnę jest dobrany ręcznie pod `TEXT_SIZE = 20`
  i wymaga korekty przy zmianie czcionki

## Stan projektu

Kod został podzielony na moduły — **struktura plików nie jest tu opisana, bo
podziału dokonano po ostatniej synchronizacji**. Przy starcie nowego czatu
warto wkleić wynik `dir src` albo sam plik, którego dotyczy rozmowa.

## Rzeczy nierozwiązane / pomysły

- Ścieżka pliku logu nie przeżywa restartu aplikacji
- Przerwy trafiają do CSV tylko sumarycznie, nie jako osobne wiersze
- Brak rotacji `nope.log`
- Długa nazwa zadania zostanie ucięta (okno ma stały rozmiar) — można dodać
  `.truncate()` albo limit długości w dialogu
- Brak podsumowania dnia w samej aplikacji

---

## Jak zacząć nowy czat

Wklej ten dokument, dopisz czego dotyczy rozmowa i **załącz konkretny plik**,
nad którym pracujesz. Przykład:

> Kontynuuję projekt opisany w handoffie poniżej. Dziś pracuję nad modułem
> logowania CSV — plik w załączniku. Chcę dodać zapis pojedynczych przerw
> jako osobnych wierszy.
