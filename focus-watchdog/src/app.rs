use eframe::egui;
use std::time::{Duration, Instant};
use chrono::Local;
use std::path::PathBuf;

use crate::task::Task;
use crate::resource::{Anim, load_anim};
use crate::utils::{fmt_minutes, parse_duration_minutes};
use crate::worklog::Worklog;
use crate::config::AppConfig;

pub enum SessionState {
    Idle,
    Working(Task),
}

/// Stan okienka dialogowego "nowe zadanie".
struct Dialog {
    name: String,
    tag: String,
    time: String,
    error: Option<String>,
    /// Czy w tej klatce ustawić fokus na pierwszym polu.
    focus_name: bool,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            name: String::new(),
            tag: String::new(),
            time: String::new(),
            error: None,
            focus_name: true,
        }
    }
}

/// Dane trybu working przygotowane do wyświetlenia.
struct WorkingView {
    name: String,
    /// Etykieta drugiego wiersza - zmienia się po wypełnieniu minimum.
    end_label: String,
    end: String,
    /// Czas od startu sesji, bez przerw. Widoczny zawsze.
    session: String,
    /// Ile zostało do wypełnienia zobowiązania. None w stanie overtime.
    remaining: Option<String>,
    /// Czy minimum jest już wypełnione (wtedy czas sesji na zielono).
    overtime: bool,
    paused: bool,
}

pub struct App {
    config: AppConfig,
    state: SessionState,
    dialog: Option<Dialog>,
    idle_anim: Anim,
    working_anim: Anim,
    overtime_anim: Anim,
    pause_anim: Anim,
    /// Plik CSV z logiem. None = logowanie nieaktywne.
    log_path: Option<PathBuf>,
    /// Komunikat po ostatniej próbie zapisu.
    log_status: Option<String>,
    worklog: Box<dyn Worklog>,
}

impl App {
    pub fn new(ctx: &egui::Context, worklog: Box<dyn Worklog>, config : AppConfig) -> Self {
        let working_anim = load_anim(ctx, "busy");
        // Brak osobnej animacji nadgodzin nie jest błędem - wtedy leci ta sama, co przy pracy.
        let overtime_anim = match load_anim(ctx, "overtime") {
            anim if anim.error.is_none() => anim,
            _ => {
                log::info!("Brak animacji overtime - używam working");
                working_anim.clone()
            }
        };

        Self {
            config,
            state: SessionState::Idle,
            dialog: None,
            idle_anim: load_anim(ctx, "idle"),
            working_anim,
            overtime_anim,
            pause_anim: load_anim(ctx, "pause"),
            log_path: None,
            log_status: None,
            worklog,
        }
    }

    fn start_task(&mut self, ctx: &egui::Context, name: String, tag: String, minutes: u64) {
        let minimum = Duration::from_secs(minutes * 60);
        let now = Local::now();
        let end_dt = now + chrono::Duration::minutes(minutes as i64);

        self.state = SessionState::Working(Task {
            name,
            tag,
            minimum,
            started: Instant::now(),
            started_at: now,
            paused_total: Duration::ZERO,
            paused_at: None,
            end_label: end_dt.format("%H:%M, %d.%m.%Y").to_string(),
            notified: false,
        });

        // Okno na wierzch na czas pracy i bez możliwości schowania go w pasek zadań.
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::EnableButtons {
            close: true,
            minimized: false,
            maximize: false,
        });
    }

    /// Zadanie kończy wyłącznie użytkownik przyciskiem "przerwij".
    fn back_to_idle(&mut self, ctx: &egui::Context) {
        self.state = SessionState::Idle;

        // Zwolnij "zawsze na wierzchu" i przywróć minimalizowanie.
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
        ctx.send_viewport_cmd(egui::ViewportCommand::EnableButtons {
            close: true,
            minimized: true,
            maximize: false,
        });
    }

    fn row(&self, ui: &mut egui::Ui, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).strong().size(self.config.text_size));
            ui.label(egui::RichText::new(value).size(self.config.text_size));
        });
    }

    /// Wiersz z wyróżnioną kolorem wartością.
    fn row_colored(&self, ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).strong().size(self.config.text_size));
            ui.label(
                egui::RichText::new(value)
                    .strong()
                    .size(self.config.text_size)
                    .color(color),
            );
        });
    }
}

impl eframe::App for App {
    // UWAGA: od eframe 0.34/0.36 wymaganą metodą jest `ui`, a nie `update`,
    // i dostajemy gotowe `&mut Ui` zamiast `&Context`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Context jest tanim uchwytem (Arc) - klonujemy, żeby nie kolidować z borrowem `ui`.
        let ctx = ui.ctx().clone();

        // 1. Minimum wypełnione? Nie kończymy zadania - tylko raz sygnalizujemy.
        if let SessionState::Working(t) = &mut self.state {
            if !t.notified && t.is_overtime() {
                t.notified = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Informational,
                ));
            }
        }

        // Awaryjne przywracanie okna, jeśli mimo wyłączonego przycisku
        // zostało zminimalizowane skrótem systemowym.
        if self.config.force_restore && matches!(self.state, SessionState::Working(_)) {
            if ctx.input(|i| i.viewport().minimized) == Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
        }

        // 2. Przygotuj dane do wyświetlenia (żeby nie walczyć z borrow checkerem w domknięciach).
        // Podczas pauzy wracamy do animacji idle - widać stan jednym rzutem oka.
        let anim = match &self.state {
            SessionState::Idle => self.idle_anim.clone(),
            SessionState::Working(t) if t.is_paused() => self.pause_anim.clone(),
            SessionState::Working(t) if t.is_overtime() => self.overtime_anim.clone(),
            SessionState::Working(_) => self.working_anim.clone(),
        };
        let working_view = match &self.state {
            SessionState::Working(t) => {
                let overtime = t.is_overtime();

                // Czas sesji biegnie zawsze, także po wypełnieniu minimum
                // i po przedłużeniu. Zaokrąglany w dół - "tyle już mam".
                let session = fmt_minutes(t.worked().as_secs() / 60);

                // Zostało tylko dopóki zobowiązanie niewypełnione.
                // W górę, żeby 44:01 pokazać jeszcze jako 45min.
                let remaining = (!overtime)
                    .then(|| fmt_minutes(t.remaining().as_secs().div_ceil(60)));

                let (end_label, end) = if overtime {
                    ("Minimum od:", t.end_label.clone())
                } else if t.is_paused() {
                    ("Koniec:", "wstrzymane".to_owned())
                } else {
                    ("Koniec:", t.end_label.clone())
                };

                Some(WorkingView {
                    name: t.name.clone(),
                    end_label: end_label.to_owned(),
                    end,
                    session,
                    remaining,
                    overtime,
                    paused: t.is_paused(),
                })
            }
            SessionState::Idle => None,
        };

        let mut open_dialog = false;
        let mut abort = false;
        let mut toggle_pause = false;
        let mut pick_log = false;
        let mut clear_log = false;
        let mut extend: Option<u64> = None;
        let log_path = self.log_path.clone();
        let log_status = self.log_status.clone();
        let anim_size = self.config.anim_size;
        let button_height = self.config.button_height;
        let overtime_color = egui::Color32::from_rgb(
                                self.config.overtime_color.0, 
                                self.config.overtime_color.1, 
                                self.config.overtime_color.2
                            );

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
                                .fit_to_exact_size(egui::vec2(anim_size, anim_size))
                                .maintain_aspect_ratio(false),
                        );
                    }
                    Some(err) => {
                        ui.allocate_ui(egui::vec2(anim_size, anim_size), |ui| {
                            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                        });
                    }
                }

                ui.add_space(18.0);

                // Prawa strona: zależnie od stanu.
                ui.vertical(|ui| {
                    // Wyśrodkowanie względem animacji: każdy stan ma inną wysokość treści.
                    ui.add_space(match &working_view {
                        None => anim_size / 2.0 - 58.0,
                        // Busy ma cztery wiersze, overtime trzy.
                        Some(v) if v.remaining.is_some() => anim_size / 2.0 - 84.0,
                        Some(_) => anim_size / 2.0 - 66.0,
                    });
                    match &working_view {
                        None => {
                            if ui
                                .add_sized([160.0, 34.0], egui::Button::new("nowe zadanie"))
                                .clicked()
                            {
                                open_dialog = true;
                            }

                            ui.add_space(14.0);

                            ui.horizontal(|ui| {
                                if ui.button("plik logu…").clicked() {
                                    pick_log = true;
                                }
                                if log_path.is_some() && ui.button("wyłącz").clicked() {
                                    clear_log = true;
                                }
                            });                            

                            match &log_path {
                                Some(path) => {
                                    let name = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| path.display().to_string());
                                    ui.label(
                                        egui::RichText::new(format!("zapis do: {name}"))
                                            .small()
                                            .color(overtime_color),
                                    )
                                    .on_hover_text(path.display().to_string());
                                }
                                None => {
                                    ui.label(
                                        egui::RichText::new("logowanie nieaktywne")
                                            .small()
                                            .weak(),
                                    );
                                }
                            }

                            if let Some(status) = &log_status {
                                ui.label(
                                    egui::RichText::new(status)
                                        .small()
                                        .color(egui::Color32::from_rgb(220, 80, 80)),
                                );
                            }
                        }
                        Some(view) => {
                            ui.spacing_mut().item_spacing.y = 8.0;
                            self.row(ui, "Bieżące zadanie:", &view.name);
                            self.row(ui, &view.end_label, &view.end);
                            if view.overtime {
                                self.row_colored(ui, "Czas sesji:", &view.session, overtime_color);
                            } else {
                                self.row(ui, "Czas sesji:", &view.session);
                            }
                            if let Some(left) = &view.remaining {
                                self.row(ui, "Pozostało:", left);
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let label = if view.paused { "wznów" } else { "pauza" };
                                if ui
                                    .add_sized([90.0, button_height], egui::Button::new(label))
                                    .clicked()
                                {
                                    toggle_pause = true;
                                }
                                if ui
                                    .add_sized([90.0, button_height], egui::Button::new("zakończ"))
                                    .clicked() 
                                {
                                    abort = true;
                                }
                                // Przedłużenie ma sens dopiero, gdy zobowiązanie wypełnione.
                                if view.overtime {
                                    // ComboBox nie przyjmuje add_sized - wysokość bierze
                                    // ze stylu. Zmiana dotyczy tylko tego wiersza.
                                    let text_h = ui.text_style_height(&egui::TextStyle::Button);
                                    ui.spacing_mut().interact_size.y = button_height;
                                    ui.spacing_mut().button_padding.y =
                                        ((button_height - text_h) / 2.0).max(0.0);

                                    egui::ComboBox::from_id_salt("extend")
                                        .selected_text("przedłuż")
                                        .width(110.0)
                                        .show_ui(ui, |ui| {
                                            for minutes in &self.config.extend_options {
                                                if ui
                                                    .selectable_label(
                                                        false,
                                                        format!("+{minutes} min"),
                                                    )
                                                    .clicked()
                                                {
                                                    extend = Some(*minutes as u64);
                                                }
                                            }
                                        });
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

                            ui.label("Tag:");
                            ui.add(
                                egui::TextEdit::singleline(&mut d.tag)
                                    .hint_text("do grupowania, opcjonalny")
                                    .desired_width(200.0),
                            );
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
            let parsed = self.dialog.as_ref().and_then(|d| {
                parse_duration_minutes(&d.time).map(|m| (d.name.clone(), d.tag.clone(), m))
            });

            match parsed {
                Some((name, tag, minutes)) => {
                    let name = if name.trim().is_empty() {
                        "(bez nazwy)".to_owned()
                    } else {
                        name.trim().to_owned()
                    };
                    self.start_task(&ctx, name, tag.trim().to_owned(), minutes);
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
            if let SessionState::Working(t) = &mut self.state {
                if t.is_paused() {
                    t.resume();
                } else {
                    t.pause();
                }
            }
        }
        if let Some(minutes) = extend {
            if let SessionState::Working(t) = &mut self.state {
                t.extend(Duration::from_secs(minutes * 60));
                log::info!("sesja przedłużona o {minutes} min");
            }
        }

        if abort {
            // Zapis PRZED zmianą stanu - potem zadania już nie ma.
            if let SessionState::Working(t) = &self.state {
                match self.worklog.add_record(t) {
                    Ok(()) => {
                        log::info!("zapisano do logu");
                        self.log_status = None;
                    }
                    Err(err) => {
                        log::error!("zapis do logu nieudany: {err}");
                        self.log_status = Some(format!("Błąd zapisu: {err}"));
                    }
                }
            }
            self.back_to_idle(&ctx);
        }

        if pick_log {
            // Dialog systemowy blokuje wątek UI aż do wyboru - to jest OK,
            // bo i tak nie ma co odświeżać w tle.
            let mut chooser = rfd::FileDialog::new()
                .set_title("Plik logu zadań")
                .add_filter("CSV", &["csv"]);
            chooser = match &self.log_path {
                Some(current) => {
                    let dir = current.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                    chooser.set_directory(dir).set_file_name(
                        current
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )
                }
                None => chooser.set_file_name("zadania.csv"),
            };
            if let Some(path) = chooser.save_file() {
                log::info!("plik logu: {}", path.display());
                self.log_path = Some(path);
                self.log_status = None;
            }
        }

        if clear_log {
            self.log_path = None;
            self.log_status = None;
        }

        // 6. Odświeżanie: obudź się dokładnie wtedy, gdy zmieni się wyświetlana minuta.
        // W stanie busy tykają dwa liczniki - bierzemy wcześniejszy z nich.
        // Podczas pauzy nic nie tyka, więc nie ma po co budzić UI.
        if let SessionState::Working(t) = &self.state {
            if !t.is_paused() {
                // Czas sesji rośnie i jest zaokrąglany w dół.
                let session_wait = 60 - t.worked().as_secs() % 60;

                let wait = if t.is_overtime() {
                    session_wait
                } else {
                    // Pozostało maleje i jest zaokrąglane w górę.
                    let secs = t.remaining().as_secs();
                    let shown_min = secs.div_ceil(60);
                    let remaining_wait = secs - 60 * shown_min.saturating_sub(1);
                    session_wait.min(remaining_wait)
                };

                ctx.request_repaint_after(Duration::from_secs(wait.clamp(1, 60)));
            }
        }
    }
}
