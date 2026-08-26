use anyhow::Result;
use std::{fs::File, path::PathBuf};
//use std::time::{Duration, Instant};
use chrono::Local;

use crate::task::Task;
use crate::utils::to_minutes;

pub struct WorklogTask {
    pub name: String,
    pub tag: String,
}

pub trait Worklog {
    fn add_record(&mut self, record: &str) -> Result<()>;
    fn get_task_names(&self) -> Result<Vec<WorklogTask>>;
    fn get_top_tasks(&self) -> Result<Vec<WorklogTask>>;
}

struct FileWorklog {
    path: PathBuf,
}

impl FileWorklog {
    pub fn new(path: PathBuf) -> Self {
        FileWorklog { path }
    }
}

impl Worklog for FileWorklog {
    fn add_record(&mut self, record: &str) -> Result<()> {
        // Implementacja dodawania rekordu do pliku
        Ok(())
    }

    fn get_task_names(&self) -> Result<Vec<WorklogTask>> {
        // Implementacja pobierania nazw zadań z pliku
        Ok(vec![])
    }

    fn get_top_tasks(&self) -> Result<Vec<WorklogTask>> {
        // Implementacja pobierania najważniejszych zadań z pliku
        Ok(vec![])
    }
}


//------------------- temporary
/// Nagłówek pisany tylko przy zakładaniu nowego (pustego) pliku.
const CSV_HEADER: &str = "started_at,ended_at,task,tag,minimum_min,worked_min,paused_min";

/// Otacza pole cudzysłowami, jeśli zawiera przecinek, cudzysłów albo nową linię.
/// Cudzysłowy w środku podwajamy - tak wymaga RFC 4180 i tak czyta to pandas.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Dopisuje jeden wiersz na koniec pliku. Zakłada plik i nagłówek, jeśli trzeba.
pub fn append_log(path: &std::path::Path, task: &Task) -> std::io::Result<()> {
    use std::io::Write as _;

    let fresh = match std::fs::metadata(path) {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    if fresh {
        writeln!(file, "{CSV_HEADER}")?;
    }

    let fmt = "%Y-%m-%d %H:%M:%S";
    writeln!(
        file,
        "{},{},{},{},{},{},{}",
        task.started_at.format(fmt),
        Local::now().format(fmt),
        csv_field(&task.name),
        csv_field(&task.tag),
        to_minutes(task.minimum),
        to_minutes(task.worked()),
        to_minutes(task.paused_time()),
    )
}