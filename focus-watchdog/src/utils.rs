/// Parsuje czas podany po ludzku i zwraca liczbę minut.
/// Rozumie: "1h 20min", "1godz 20min", "45min", "2h", "90" (gołe liczby = minuty), "1:30".
use std::time::Duration;

pub fn parse_duration_minutes(input: &str) -> Option<u64> {
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

pub fn fmt_minutes(mins: u64) -> String {
    let h = mins / 60;
    let m = mins % 60;
    match (h, m) {
        (0, m) => format!("{m}min"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}min"),
    }
}

/// Sekundy na minuty, zaokrąglane do najbliższej.
pub fn to_minutes(d: Duration) -> u64 {
    (d.as_secs() + 30) / 60
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
