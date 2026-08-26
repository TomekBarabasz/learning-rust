use std::time::{Duration, Instant};
use chrono::Local;

pub struct Task {
    pub name: String,
    /// Etykieta do grupowania zadań w analizie. Może być pusta.
    pub tag: String,
    /// Zadeklarowane minimum - po jego wypełnieniu liczymy dalej.
    pub minimum: Duration,
    /// Moment startu zadania.
    pub started: Instant,
    /// Moment startu wg zegara ściennego - do logu.
    pub started_at: chrono::DateTime<Local>,
    /// Suma zakończonych przerw.
    pub paused_total: Duration,
    /// Początek trwającej przerwy, jeśli akurat stoimy.
    pub paused_at: Option<Instant>,
    /// Data i godzina wypełnienia minimum - gotowa do wyświetlenia.
    pub end_label: String,
    /// Czy zdążyliśmy już zasygnalizować osiągnięcie minimum.
    pub notified: bool,
}

impl Task {
    pub fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }

    /// Faktycznie przepracowany czas, bez przerw. Podczas pauzy nie rośnie,
    /// bo trwająca przerwa odejmuje się dokładnie tak szybko, jak przyrasta zegar.
    pub fn worked(&self) -> Duration {
        let paused = self.paused_total
            + self
                .paused_at
                .map(|since| since.elapsed())
                .unwrap_or_default();
        self.started.elapsed().saturating_sub(paused)
    }

    /// Łączny czas przerw, wliczając trwającą.
    pub fn paused_time(&self) -> Duration {
        self.paused_total
            + self
                .paused_at
                .map(|since| since.elapsed())
                .unwrap_or_default()
    }

    /// Ile brakuje do minimum. Zero oznacza, że minimum wypełnione.
    pub fn remaining(&self) -> Duration {
        self.minimum.saturating_sub(self.worked())
    }

    /// Czy pracujemy już ponad zadeklarowane minimum.
    pub fn is_overtime(&self) -> bool {
        self.worked() >= self.minimum
    }

    /// Przedłuża zobowiązanie o zadany czas, licząc od teraz.
    /// Sesja biegnie dalej bez resetu - zmienia się tylko to, do czego się zobowiązujemy.
    pub fn extend(&mut self, extra: Duration) {
        self.minimum = self.worked() + extra;
        self.notified = false;
        self.refresh_end_label();
    }

    pub fn pause(&mut self) {
        if !self.is_paused() {
            self.paused_at = Some(Instant::now());
        }
    }

    pub fn resume(&mut self) {
        if let Some(since) = self.paused_at.take() {
            self.paused_total += since.elapsed();
            // Moment wypełnienia minimum przesuwa się o całą długość przerwy.
            self.refresh_end_label();
        }
    }

    /// Przelicza godzinę, o której minimum zostanie wypełnione.
    pub fn refresh_end_label(&mut self) {
        let left = self.remaining();
        let end_dt = Local::now()
            + chrono::Duration::from_std(left).unwrap_or_else(|_| chrono::Duration::zero());
        self.end_label = end_dt.format("%H:%M, %d.%m.%Y").to_string();
    }
}
