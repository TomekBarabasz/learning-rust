use eframe::egui;
use std::time::{Duration, Instant};
use chrono::Local;

use crate::task::Task;
use crate::resource::{Anim, load_anim};
use crate::utils::{fmt_minutes, parse_duration_minutes};
use crate::worklog::{Worklog,WorklogTask};
use crate::config::AppConfig;

/// Kolor komunikatów o błędach (brak animacji, zły format czasu, zapis do logu).
const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 80, 80);

pub enum SessionState {
    Idle,
    Working(Task),
}

/// Co użytkownik zrobił w tej klatce.
///
/// Rysowanie tylko zaznacza flagi, a zmiany stanu dzieją się później,
/// w `apply`. Dzięki temu domknięcia egui nie muszą trzymać `&mut self`
/// i nie ma walki z borrow checkerem.
#[derive(Default)]
struct Actions {
    open_dialog: bool,
    submit: bool,
    cancel: bool,
    toggle_pause: bool,
    extend: Option<u64>,
    abort: bool,
}

const NAME_FIELD_WIDTH: f32 = 380.0;
const SUGGEST_WIDTH: f32 = 110.0;

struct Dialog {
    name: String,
    tag: String,
    time: String,
    error: Option<String>,
    focus_name: bool,
    suggestions: Vec<WorklogTask>,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            name: String::new(),
            tag: String::new(),
            time: String::new(),
            error: None,
            focus_name: true,
            suggestions: Vec::new()
        }
    }
}

impl Dialog {
    fn new(suggestions: Vec<WorklogTask>) -> Self {
        Self {
            suggestions,
            ..Default::default()
        }
    }

    /// Rysuje okno dialogowe. Window nadal pokazuje się przez Context.
    fn show(&mut self, ctx: &egui::Context, act: &mut Actions) {
        let mut window_open = true;

        egui::Window::new("Nowe zadanie")
            .collapsible(false)
            .resizable(false)
            .min_width(500.0)
            .open(&mut window_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                self.draw_form(ui);

                if let Some(err) = &self.error {
                    ui.add_space(4.0);
                    ui.colored_label(ERROR_COLOR, err);
                }

                ui.add_space(6.0);
                ui.separator();
                Self::draw_buttons(ui, act);

                // Enter = start, Esc = anuluj.
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    act.submit = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    act.cancel = true;
                }
            });

        // Zamknięcie krzyżykiem traktujemy jak anulowanie.
        if !window_open {
            act.cancel = true;
        }
    }

    /// Wybór z listy wypełnia nazwę i tag. Oba zostają zwykłymi polami,
    /// więc użytkownik może je potem poprawić.
    fn draw_suggestions(&mut self, ui: &mut egui::Ui) {
        if self.suggestions.is_empty() {
            return;
        }

        fn suggestion_label(task: &WorklogTask) -> String {
            if task.tag.is_empty() {
                task.name.clone()
            } else {
                format!("{} · {}", task.name, task.tag)
            }
        }


        let mut chosen: Option<usize> = None;

        egui::ComboBox::from_id_salt("task-suggestions")
            .selected_text("poprzednie")
            .width(SUGGEST_WIDTH)
            .show_ui(ui, |ui| {
                for (index, task) in self.suggestions.iter().enumerate() {
                    if ui.selectable_label(false, suggestion_label(task)).clicked() {
                        chosen = Some(index);
                    }
                }
            });

        if let Some(index) = chosen {
            let (name, tag) = {
                let task = &self.suggestions[index];
                (task.name.clone(), task.tag.clone())
            };
            self.name = name;
            self.tag = tag;
        }
    }

    /// Trzy pola formularza w siatce dwukolumnowej.
    fn draw_form(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("form")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("Nazwa zadania:");
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.name)
                            .desired_width(NAME_FIELD_WIDTH - SUGGEST_WIDTH - 8.0),
                    );
                    if self.focus_name {
                        resp.request_focus();
                        self.focus_name = false;
                    }
                    self.draw_suggestions(ui);
                });
                ui.end_row();

                ui.label("Tag:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.tag)
                        .hint_text("do grupowania, opcjonalny")
                        .desired_width(200.0),
                );
                ui.end_row();

                ui.label("Czas:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.time)
                        .hint_text("np. 1h 20min")
                        .desired_width(200.0),
                );
                ui.end_row();
            });
    }

    fn draw_buttons(ui: &mut egui::Ui, act: &mut Actions) {
        ui.horizontal(|ui| {
            if ui
                .add_sized([80.0, 26.0], egui::Button::new("start"))
                .clicked()
            {
                act.submit = true;
            }
            if ui.button("anuluj").clicked() {
                act.cancel = true;
            }
        });
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

impl WorkingView {
    fn from_task(t: &Task) -> Self {
        let overtime = t.is_overtime();

        // Czas sesji biegnie zawsze, także po wypełnieniu minimum
        // i po przedłużeniu. Zaokrąglany w dół - "tyle już mam".
        let session = fmt_minutes(t.worked().as_secs() / 60);

        // Zostało tylko dopóki zobowiązanie niewypełnione.
        // W górę, żeby 44:01 pokazać jeszcze jako 45min.
        let remaining = (!overtime).then(|| fmt_minutes(t.remaining().as_secs().div_ceil(60)));

        let (end_label, end) = if overtime {
            ("Minimum od:", t.end_label.clone())
        } else if t.is_paused() {
            ("Koniec:", "wstrzymane".to_owned())
        } else {
            ("Koniec:", t.end_label.clone())
        };

        Self {
            name: t.name.clone(),
            end_label: end_label.to_owned(),
            end,
            session,
            remaining,
            overtime,
            paused: t.is_paused(),
        }
    }
}

pub struct App {
    config: AppConfig,
    state: SessionState,
    dialog: Option<Dialog>,
    idle_anim: Anim,
    working_anim: Anim,
    overtime_anim: Anim,
    pause_anim: Anim,
    worklog: Box<dyn Worklog>,
}

impl App {
    pub fn new(ctx: &egui::Context, worklog: Box<dyn Worklog>, config: AppConfig) -> Self {
        Self {
            config,
            state: SessionState::Idle,
            dialog: None,
            idle_anim: load_anim(ctx, "idle"),
            working_anim: load_anim(ctx, "busy"),
            overtime_anim: load_anim(ctx, "overtime"),
            pause_anim: load_anim(ctx, "pause"),
            worklog,
        }
    }

    fn overtime_color(&self) -> egui::Color32 {
        let (r, g, b) = self.config.overtime_color;
        egui::Color32::from_rgb(r, g, b)
    }

    // --- przejścia między stanami ------------------------------------------

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

    /// Zadanie kończy wyłącznie użytkownik przyciskiem "zakończ".
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

    // --- rzeczy robione na starcie klatki ----------------------------------

    /// Minimum wypełnione? Nie kończymy zadania - tylko raz sygnalizujemy.
    fn notify_if_minimum_done(&mut self, ctx: &egui::Context) {
        if let SessionState::Working(t) = &mut self.state {
            if !t.notified && t.is_overtime() {
                t.notified = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Informational,
                ));
            }
        }
    }

    /// Awaryjne przywracanie okna, jeśli mimo wyłączonego przycisku
    /// zostało zminimalizowane skrótem systemowym.
    fn keep_window_visible(&self, ctx: &egui::Context) {
        if cfg!(feature = "kiosk") && matches!(self.state, SessionState::Working(_)) {
            if ctx.input(|i| i.viewport().minimized) == Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
        }
    }

    // --- rysowanie ---------------------------------------------------------

    /// Animacja dobrana do stanu. Podczas pauzy własna - widać stan
    /// jednym rzutem oka.
    fn current_anim(&self) -> Anim {
        match &self.state {
            SessionState::Idle => self.idle_anim.clone(),
            SessionState::Working(t) if t.is_paused() => self.pause_anim.clone(),
            SessionState::Working(t) if t.is_overtime() => self.overtime_anim.clone(),
            SessionState::Working(_) => self.working_anim.clone(),
        }
    }

    /// Okno główne: animacja po lewej, treść zależna od stanu po prawej.
    fn draw_main_panel(&self, ui: &mut egui::Ui, act: &mut Actions) {
        let anim = self.current_anim();
        let view = match &self.state {
            SessionState::Working(t) => Some(WorkingView::from_task(t)),
            SessionState::Idle => None,
        };

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                self.draw_anim(ui, &anim);
                ui.add_space(18.0);

                ui.vertical(|ui| {
                    match &view {
                        None => self.draw_idle_pane(ui, act),
                        Some(view) => self.draw_working_pane(ui, view, act),
                    }
                });
            });
        });
    }

    /// Kwadratowa animacja albo komunikat, czemu jej nie ma.
    fn draw_anim(&self, ui: &mut egui::Ui, anim: &Anim) {
        let size = self.config.anim_size;
        match &anim.error {
            None => {
                ui.add(
                    egui::Image::new(anim.uri.as_str())
                        .fit_to_exact_size(egui::vec2(size, size))
                        .maintain_aspect_ratio(false),
                );
            }
            Some(err) => {
                ui.allocate_ui(egui::vec2(size, size), |ui| {
                    ui.colored_label(ERROR_COLOR, err);
                });
            }
        }
    }

    fn draw_idle_pane(&self, ui: &mut egui::Ui, act: &mut Actions) {
        if ui
            .add_sized([160.0, 34.0], egui::Button::new("nowe zadanie"))
            .clicked()
        {
            act.open_dialog = true;
        }

        ui.add_space(14.0);
        ui.label(egui::RichText::new(self.worklog.get_name()).small().weak());
    }

    // --- prawa kolumna: praca ----------------------------------------------

    fn draw_working_pane(&self, ui: &mut egui::Ui, view: &WorkingView, act: &mut Actions) {
        ui.spacing_mut().item_spacing.y = 8.0;
        self.draw_working_rows(ui, view);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            self.draw_working_buttons(ui, view, act);
        });
    }
    
    
    fn draw_name(&self, ui: &mut egui::Ui, name: &str) {
        let font = egui::FontId::proportional(self.config.text_size);
        let strong = ui.visuals().strong_text_color();
        let normal = ui.visuals().text_color();
        const NAME_WIDTH: f32 = 320.0;
        let mut job = egui::text::LayoutJob::default();
        job.wrap = egui::text::TextWrapping {
            max_width: NAME_WIDTH,
            max_rows: 3,
            // Długie słowo bez spacji ma się złamać, a nie wyjechać poza okno.
            break_anywhere: false,
            ..Default::default()
        };

        job.append(
            "Bieżące zadanie:  ",
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color: strong,
                ..Default::default()
            },
        );
        job.append(
            name,
            0.0,
            egui::TextFormat {
                font_id: font,
                color: normal,
                ..Default::default()
            },
        );

        ui.add(egui::Label::new(job));
    }

    fn draw_working_rows(&self, ui: &mut egui::Ui, view: &WorkingView) {
        self.draw_name(ui, &view.name);
        self.row(ui, &view.end_label, &view.end);

        if view.overtime {
            self.row_colored(ui, "Czas sesji:", &view.session, self.overtime_color());
        } else {
            self.row(ui, "Czas sesji:", &view.session);
        }

        if let Some(left) = &view.remaining {
            self.row(ui, "Pozostało:", left);
        }
    }

    fn draw_working_buttons(&self, ui: &mut egui::Ui, view: &WorkingView, act: &mut Actions) {
        let h = self.config.button_height;

        let label = if view.paused { "wznów" } else { "pauza" };
        if ui.add_sized([90.0, h], egui::Button::new(label)).clicked() {
            act.toggle_pause = true;
        }
        if ui
            .add_sized([90.0, h], egui::Button::new("zakończ"))
            .clicked()
        {
            act.abort = true;
        }

        // Przedłużenie ma sens dopiero, gdy zobowiązanie wypełnione.
        if view.overtime {
            self.draw_extend_combo(ui, act);
        }
    }

    fn draw_extend_combo(&self, ui: &mut egui::Ui, act: &mut Actions) {
        // ComboBox nie przyjmuje add_sized - wysokość bierze ze stylu.
        // Zmiana dotyczy tylko tego wiersza.
        let text_h = ui.text_style_height(&egui::TextStyle::Button);
        ui.spacing_mut().interact_size.y = self.config.button_height;
        ui.spacing_mut().button_padding.y = ((self.config.button_height - text_h) / 2.0).max(0.0);

        egui::ComboBox::from_id_salt("extend")
            .selected_text("przedłuż")
            .width(110.0)
            .show_ui(ui, |ui| {
                for minutes in &self.config.extend_options {
                    if ui
                        .selectable_label(false, format!("+{minutes} min"))
                        .clicked()
                    {
                        act.extend = Some(*minutes as u64);
                    }
                }
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

    // --- reakcje na akcje użytkownika --------------------------------------

    fn apply(&mut self, ctx: &egui::Context, act: Actions) {
        self.apply_dialog(ctx, &act);
        self.apply_task(ctx, &act);
    }

    fn apply_dialog(&mut self, ctx: &egui::Context, act: &Actions) {
        if act.open_dialog {
            self.dialog = Some(Dialog::new(self.worklog.get_recent_tasks()));
        }
        if act.cancel {
            self.dialog = None;
        }
        if act.submit {
            self.submit_dialog(ctx);
        }
    }

    /// Próba startu zadania z danych z dialogu. Zły czas = komunikat w dialogu,
    /// okno zostaje otwarte.
    fn submit_dialog(&mut self, ctx: &egui::Context) {
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
                self.start_task(ctx, name, tag.trim().to_owned(), minutes);
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

    fn apply_task(&mut self, ctx: &egui::Context, act: &Actions) {
        if act.toggle_pause {
            if let SessionState::Working(t) = &mut self.state {
                if t.is_paused() {
                    t.resume();
                } else {
                    t.pause();
                }
            }
        }

        if let Some(minutes) = act.extend {
            if let SessionState::Working(t) = &mut self.state {
                t.extend(Duration::from_secs(minutes * 60));
                log::info!("sesja przedłużona o {minutes} min");
            }
        }

        if act.abort {
            self.finish_task(ctx);
        }
    }

    /// Zapis PRZED zmianą stanu - potem zadania już nie ma.
    fn finish_task(&mut self, ctx: &egui::Context) {
        if let SessionState::Working(t) = &self.state {
            match self.worklog.add_record(t) {
                Ok(()) => {
                    log::info!("zapisano do logu");
                }
                Err(err) => {
                    log::error!("zapis do logu nieudany: {err}");
                }
            }
        }
        self.back_to_idle(ctx);
    }

    // --- odświeżanie -------------------------------------------------------

    /// Obudź się dokładnie wtedy, gdy zmieni się wyświetlana minuta.
    /// Podczas pauzy nic nie tyka, więc nie ma po co budzić UI.
    fn schedule_repaint(&self, ctx: &egui::Context) {
        if let SessionState::Working(t) = &self.state {
            if !t.is_paused() {
                ctx.request_repaint_after(Duration::from_secs(next_tick_secs(t)));
            }
        }
    }
}

/// Ile sekund do najbliższej zmiany któregokolwiek z wyświetlanych liczników.
/// W stanie busy tykają dwa naraz - bierzemy wcześniejszy z nich.
fn next_tick_secs(t: &Task) -> u64 {
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

    wait.clamp(1, 60)
}

impl eframe::App for App {
    // UWAGA: od eframe 0.34/0.36 wymaganą metodą jest `ui`, a nie `update`,
    // i dostajemy gotowe `&mut Ui` zamiast `&Context`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Context jest tanim uchwytem (Arc) - klonujemy, żeby nie kolidować z borrowem `ui`.
        let ctx = ui.ctx().clone();

        self.notify_if_minimum_done(&ctx);
        self.keep_window_visible(&ctx);

        let mut act = Actions::default();
        self.draw_main_panel(ui, &mut act);
        if let Some(dialog) = self.dialog.as_mut() {
            dialog.show(&ctx, &mut act);
        }

        self.apply(&ctx, act);
        self.schedule_repaint(&ctx);
    }
}
