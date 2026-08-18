use std::collections::HashSet;
pub fn check(candidate: &str) -> bool {
    let cleaned : Vec<char> = candidate.chars()
            .filter_map(|c| if c.is_alphabetic() { Some(c.to_ascii_lowercase()) } else {None})
            .collect();
    let unique : HashSet<char> = cleaned.iter().copied().collect();
    cleaned.len() == unique.len()
}
pub fn check_1(candidate: &str) -> bool {
    let cleaned : Vec<char> = candidate.chars()
            .filter(|c| c.is_alphabetic())
            .map(|c| c.to_ascii_lowercase())
            .collect();
    let unique : HashSet<char> = cleaned.iter().copied().collect();
    cleaned.len() == unique.len()
}
pub fn check_2(candidate: &str) -> bool {
    let mut seen = HashSet::new();
    candidate
        .chars()
        .filter(|c| c.is_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .all(|c| seen.insert(c))
}