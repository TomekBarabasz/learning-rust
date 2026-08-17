// Ukryj konsolę w buildzie release na Windows (w debug zostaje - przydaje się do logów).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Local;
use eframe::egui;

/// Bok kwadratowej animacji w pikselach.
const ANIM_SIZE: f32 = 200.0;

/// Wielkość czcionki w trybie working (domyślna w egui to ok. 14).
const TEXT_SIZE: f32 = 20.0;

/// Ikona okna (pasek zadań, Alt+Tab). Kwadratowy PNG, najlepiej 256x256.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Animacje wkompilowane w binarkę na etapie `cargo build`.
///
/// Ścieżki są względne do tego pliku (`src/main.rs`), więc pliki muszą istnieć
/// pod `assets/` już w czasie kompilacji - inaczej dostaniesz błąd kompilatora.
/// Zmieniasz format? Popraw i rozszerzenie w pierwszej kolumnie, i ścieżkę.
const EMBEDDED: &[(&str, &str, &[u8])] = &[
    ("idle", "gif", include_bytes!("../assets/idle.gif")),
    ("busy", "gif", include_bytes!("../assets/busy.gif")),
    ("pause", "gif", include_bytes!("../assets/pause.gif")),
];

fn main() -> eframe::Result {
    // Loadery egui zgłaszają błędy przez `log`. Bez zainicjowanego loggera
    // komunikaty typu "nie znalazłem pliku" znikają w próżni.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([640.0, 240.0])
        .with_resizable(false)
        .with_maximize_button(false)
        .with_title("Task timer");

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Task timer",
        options,
        Box::new(|cc| {
            // Bez tego obrazki się nie załadują.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}

// ---------------------------------------------------------------- stan aplikacji

struct Task {
    name: String,
    /// Moment zakończenia - do odliczania. Ma sens tylko gdy zadanie chodzi.
    end: Instant,
    /// Data i godzina zakończenia - gotowa do wyświetlenia.
    end_label: String,
    /// Gdy wstrzymane: ile czasu zostało w chwili naciśnięcia pauzy.
    paused_left: Option<Duration>,
}

impl Task {
    fn is_paused(&self) -> bool {
        self.paused_left.is_some()
    }

    /// Ile jeszcze zostało. Podczas pauzy wartość zamrożona.
    fn remaining(&self) -> Duration {
        match self.paused_left {
            Some(left) => left,
            None => self.end.saturating_duration_since(Instant::now()),
        }
    }

    fn pause(&mut self) {
        if !self.is_paused() {
            self.paused_left = Some(self.remaining());
        }
    }

    fn resume(&mut self) {
        if let Some(left) = self.paused_left.take() {
            // Koniec przesuwa się o całą długość przerwy.
            self.end = Instant::now() + left;
            let end_dt = Local::now()
                + chrono::Duration::from_std(left).unwrap_or_else(|_| chrono::Duration::zero());
            self.end_label = end_dt.format("%H:%M, %d.%m.%Y").to_string();
        }
    }
}

enum State {
    Idle,
    Working(Task),
}

/// Stan okienka dialogowego "nowe zadanie".
struct Dialog {
    name: String,
    time: String,
    error: Option<String>,
    /// Czy w tej klatce ustawić fokus na pierwszym polu.
    focus_name: bool,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            name: String::new(),
            time: String::new(),
            error: None,
            focus_name: true,
        }
    }
}

/// Animacja wczytana do pamięci albo informacja, czemu się nie udało.
#[derive(Clone)]
struct Anim {
    /// URI w schemacie `bytes://`, pod którym bajty siedzą w cache egui.
    uri: String,
    /// Komunikat do pokazania zamiast obrazka.
    error: Option<String>,
}

/// Dane trybu working przygotowane do wyświetlenia.
struct WorkingView {
    name: String,
    end: String,
    left: String,
    paused: bool,
}

struct App {
    state: State,
    dialog: Option<Dialog>,
    idle_anim: Anim,
    working_anim: Anim,
    pause_anim: Anim,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        Self {
            state: State::Idle,
            dialog: None,
            idle_anim: load_anim(ctx, "idle"),
            working_anim: load_anim(ctx, "busy"),
            pause_anim: load_anim(ctx, "pause"),
        }
    }

    fn start_task(&mut self, ctx: &egui::Context, name: String, minutes: u64) {
        let end = Instant::now() + Duration::from_secs(minutes * 60);
        let end_dt = Local::now() + chrono::Duration::minutes(minutes as i64);

        self.state = State::Working(Task {
            name,
            end,
            end_label: end_dt.format("%H:%M, %d.%m.%Y").to_string(),
            paused_left: None,
        });

        // Okno na wierzch na czas pracy.
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
    }

    fn back_to_idle(&mut self, ctx: &egui::Context, finished: bool) {
        self.state = State::Idle;

        // Zwolnij "zawsze na wierzchu" - okno może znów schować się pod inne.
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));

        if finished {
            // Delikatne mrugnięcie w pasku zadań, żeby nie przegapić końca.
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                egui::UserAttentionType::Informational,
            ));
        }
    }
}

impl eframe::App for App {
    // UWAGA: od eframe 0.34/0.36 wymaganą metodą jest `ui`, a nie `update`,
    // i dostajemy gotowe `&mut Ui` zamiast `&Context`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Context jest tanim uchwytem (Arc) - klonujemy, żeby nie kolidować z borrowem `ui`.
        let ctx = ui.ctx().clone();

        // 1. Czy czas minął? Wstrzymane zadanie nigdy nie kończy się samo.
        let finished = match &self.state {
            State::Working(t) => !t.is_paused() && t.remaining().is_zero(),
            State::Idle => false,
        };
        if finished {
            self.back_to_idle(&ctx, true);
        }

        // 2. Przygotuj dane do wyświetlenia (żeby nie walczyć z borrow checkerem w domknięciach).
        // Podczas pauzy wracamy do animacji idle - widać stan jednym rzutem oka.
        let anim = match &self.state {
            State::Idle => self.idle_anim.clone(),
            State::Working(t) if t.is_paused() => self.pause_anim.clone(),
            State::Working(_) => self.working_anim.clone(),
        };
        let working_view = match &self.state {
            State::Working(t) => {
                // Zaokrąglamy w górę: 44:01 pokazujemy jeszcze jako 45min.
                let left_min = t.remaining().as_secs().div_ceil(60);
                let end = if t.is_paused() {
                    "wstrzymane".to_owned()
                } else {
                    t.end_label.clone()
                };
                Some(WorkingView {
                    name: t.name.clone(),
                    end,
                    left: fmt_minutes(left_min),
                    paused: t.is_paused(),
                })
            }
            State::Idle => None,
        };

        let mut open_dialog = false;
        let mut abort = false;
        let mut toggle_pause = false;

        // 3. Okno główne.
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);

                // Lewa strona: kwadratowa animacja albo komunikat, czemu jej nie ma.
                match &anim.error {
                    None => {
                        ui.add(
                            egui::Image::new(anim.uri.as_str())
                                .fit_to_exact_size(egui::vec2(ANIM_SIZE, ANIM_SIZE))
                                .maintain_aspect_ratio(false),
                        );
                    }
                    Some(err) => {
                        ui.allocate_ui(egui::vec2(ANIM_SIZE, ANIM_SIZE), |ui| {
                            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                        });
                    }
                }

                ui.add_space(18.0);

                // Prawa strona: zależnie od stanu.
                ui.vertical(|ui| {
                    ui.add_space(ANIM_SIZE / 2.0 - 46.0);
                    match &working_view {
                        None => {
                            if ui
                                .add_sized([160.0, 34.0], egui::Button::new("nowe zadanie"))
                                .clicked()
                            {
                                open_dialog = true;
                            }
                        }
                        Some(view) => {
                            ui.spacing_mut().item_spacing.y = 8.0;
                            row(ui, "Bieżące zadanie:", &view.name);
                            row(ui, "Koniec:", &view.end);
                            row(ui, "Pozostało:", &view.left);
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let label = if view.paused { "wznów" } else { "pauza" };
                                if ui
                                    .add_sized([90.0, 28.0], egui::Button::new(label))
                                    .clicked()
                                {
                                    toggle_pause = true;
                                }
                                if ui
                                    .add_sized([90.0, 28.0], egui::Button::new("zakończ"))
                                    .clicked() 
                                {
                                    abort = true;
                                }
                            });
                        }
                    }
                });
            });
        });

        // 4. Dialog "nowe zadanie" - Window nadal pokazuje się przez Context.
        let mut submit = false;
        let mut cancel = false;

        if let Some(d) = self.dialog.as_mut() {
            let mut window_open = true;
            egui::Window::new("Nowe zadanie")
                .collapsible(false)
                .resizable(false)
                .open(&mut window_open)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(&ctx, |ui| {
                    egui::Grid::new("form")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Nazwa zadania:");
                            let resp =
                                ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(200.0));
                            if d.focus_name {
                                resp.request_focus();
                                d.focus_name = false;
                            }
                            ui.end_row();

                            ui.label("Czas:");
                            ui.add(
                                egui::TextEdit::singleline(&mut d.time)
                                    .hint_text("np. 1h 20min")
                                    .desired_width(200.0),
                            );
                            ui.end_row();
                        });

                    if let Some(err) = &d.error {
                        ui.add_space(4.0);
                        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized([80.0, 26.0], egui::Button::new("start"))
                            .clicked()
                        {
                            submit = true;
                        }
                        if ui.button("anuluj").clicked() {
                            cancel = true;
                        }
                    });

                    // Enter = start, Esc = anuluj.
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        submit = true;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        cancel = true;
                    }
                });

            if !window_open {
                cancel = true;
            }
        }

        // 5. Reakcje na akcje użytkownika.
        if open_dialog {
            self.dialog = Some(Dialog::default());
        }
        if cancel {
            self.dialog = None;
        }
        if submit {
            let parsed = self
                .dialog
                .as_ref()
                .and_then(|d| parse_duration_minutes(&d.time).map(|m| (d.name.clone(), m)));

            match parsed {
                Some((name, minutes)) => {
                    let name = if name.trim().is_empty() {
                        "(bez nazwy)".to_owned()
                    } else {
                        name.trim().to_owned()
                    };
                    self.start_task(&ctx, name, minutes);
                    self.dialog = None;
                }
                None => {
                    if let Some(d) = self.dialog.as_mut() {
                        d.error = Some(
                            "Nie rozumiem czasu. Wpisz np. \"1h 20min\", \"45min\" albo \"1:30\"."
                                .to_owned(),
                        );
                    }
                }
            }
        }
        if toggle_pause {
            if let State::Working(t) = &mut self.state {
                if t.is_paused() {
                    t.resume();
                } else {
                    t.pause();
                }
            }
        }
        if abort {
            self.back_to_idle(&ctx, false);
        }

        // 6. Odświeżanie: obudź się dokładnie wtedy, gdy zmieni się wyświetlana minuta.
        // Podczas pauzy nic nie tyka, więc nie ma po co budzić UI.
        if let State::Working(t) = &self.state {
            if !t.is_paused() {
                let secs = t.remaining().as_secs();
                let shown_min = secs.div_ceil(60);
                let wait = secs - 60 * shown_min.saturating_sub(1);
                ctx.request_repaint_after(Duration::from_secs(wait.clamp(1, 60)));
            }
        }
    }
}

// ---------------------------------------------------------------- pomocnicze

/// Dekoduje wbudowany PNG do formatu, którego oczekuje egui.
/// Zwraca None zamiast panikować - brak ikony to nie powód, żeby apka nie wstała.
fn load_icon() -> Option<egui::IconData> {
    match image::load_from_memory(ICON_PNG) {
        Ok(img) => {
            let img = img.into_rgba8();
            let (width, height) = img.dimensions();
            Some(egui::IconData {
                rgba: img.into_raw(),
                width,
                height,
            })
        }
        Err(err) => {
            log::error!("Nie mogę zdekodować ikony: {err}");
            None
        }
    }
}

fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).strong().size(TEXT_SIZE));
        ui.label(egui::RichText::new(value).size(TEXT_SIZE));
    });
}

fn fmt_minutes(mins: u64) -> String {
    let h = mins / 60;
    let m = mins % 60;
    match (h, m) {
        (0, m) => format!("{m}min"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}min"),
    }
}

/// Wczytuje animację o podanej nazwie bazowej.
///
/// Kolejność: najpierw plik z dysku (obok exe, potem w katalogu roboczym,
/// rozszerzenia .webp / .gif / .png), a jeśli go nie ma - wersja wkompilowana
/// w binarkę. Dzięki temu domyślnie wystarczy sam exe, ale można podmienić
/// animację bez rekompilacji, wrzucając plik do `assets/` obok niego.
///
/// Bajty trafiają wprost do cache egui pod URI `bytes://nazwa.rozszerzenie`.
/// Dzięki temu omijamy loader `file://` i całą zabawę ze ścieżkami na Windows -
/// plik czytamy sami i od razu wiemy, czy się udało.
fn load_anim(ctx: &egui::Context, stem: &str) -> Anim {
    const EXTENSIONS: [&str; 3] = ["webp", "gif", "png"];

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("assets"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets"));
    }

    let mut tried: Vec<String> = Vec::new();

    for dir in &dirs {
        for ext in EXTENSIONS {
            let path = dir.join(format!("{stem}.{ext}"));
            if !path.is_file() {
                tried.push(path.display().to_string());
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let uri = format!("bytes://{stem}.{ext}");
                    log::info!(
                        "Wczytano {} ({} bajtów) jako {uri}",
                        path.display(),
                        bytes.len()
                    );
                    ctx.include_bytes(uri.clone(), bytes);
                    return Anim { uri, error: None };
                }
                Err(err) => {
                    log::error!("Nie mogę przeczytać {}: {err}", path.display());
                    tried.push(format!("{} ({err})", path.display()));
                }
            }
        }
    }

    // Nic na dysku - użyj wersji wkompilowanej w binarkę.
    for (name, ext, bytes) in EMBEDDED {
        if *name == stem {
            let uri = format!("bytes://{stem}.{ext}");
            log::info!("Używam wbudowanej animacji {uri} ({} bajtów)", bytes.len());
            ctx.include_bytes(uri.clone(), *bytes);
            return Anim { uri, error: None };
        }
    }

    let msg = format!("Brak pliku {stem}.(webp|gif|png).\nSzukałem w:\n{}", tried.join("\n"));
    log::error!("{msg}");
    Anim {
        uri: String::new(),
        error: Some(msg),
    }
}

/// Parsuje czas podany po ludzku i zwraca liczbę minut.
///
/// Rozumie: "1h 20min", "1godz 20min", "45min", "2h", "90" (gołe liczby = minuty), "1:30".
fn parse_duration_minutes(input: &str) -> Option<u64> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    // Wariant "hh:mm".
    if let Some((h, m)) = s.split_once(':') {
        let h: u64 = h.trim().parse().ok()?;
        let m: u64 = m.trim().parse().ok()?;
        if m >= 60 {
            return None;
        }
        let total = h * 60 + m;
        return (total > 0).then_some(total);
    }

    let mut total: u64 = 0;
    let mut pending: Option<u64> = None;
    let mut chars = s.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            let mut n: u64 = 0;
            while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
                n = n.checked_mul(10)?.checked_add(d as u64)?;
                chars.next();
            }
            // Dwie liczby pod rząd bez jednostki - nie zgadujemy.
            if pending.is_some() {
                return None;
            }
            pending = Some(n);
        } else if c.is_alphabetic() {
            let mut unit = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphabetic() {
                    unit.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let n = pending.take()?;
            match unit.as_str() {
                "h" | "hr" | "hrs" | "hour" | "hours" | "g" | "godz" | "godzin" | "godziny"
                | "godzina" => total = total.checked_add(n.checked_mul(60)?)?,
                "m" | "min" | "mins" | "minut" | "minuty" | "minuta" | "minutes" => {
                    total = total.checked_add(n)?
                }
                _ => return None,
            }
        } else if c.is_whitespace() {
            chars.next();
        } else {
            return None;
        }
    }

    // Liczba bez jednostki na końcu - traktujemy jako minuty ("1h 20", "90").
    if let Some(n) = pending {
        total = total.checked_add(n)?;
    }

    (total > 0).then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        assert_eq!(parse_duration_minutes("1h 20min"), Some(80));
        assert_eq!(parse_duration_minutes("45min"), Some(45));
        assert_eq!(parse_duration_minutes("2h"), Some(120));
        assert_eq!(parse_duration_minutes("1godz 5min"), Some(65));
        assert_eq!(parse_duration_minutes("1:30"), Some(90));
        assert_eq!(parse_duration_minutes("90"), Some(90));
        assert_eq!(parse_duration_minutes("1h 20"), Some(80));
        assert_eq!(parse_duration_minutes(""), None);
        assert_eq!(parse_duration_minutes("jutro"), None);
        assert_eq!(parse_duration_minutes("0min"), None);
    }

    #[test]
    fn formatting() {
        assert_eq!(fmt_minutes(80), "1h 20min");
        assert_eq!(fmt_minutes(45), "45min");
        assert_eq!(fmt_minutes(120), "2h");
        assert_eq!(fmt_minutes(0), "0min");
    }
}