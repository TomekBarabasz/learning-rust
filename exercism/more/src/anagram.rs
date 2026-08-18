use std::collections::HashSet;
use itertools::Itertools;

fn _1_anagrams_for<'a>(word: &str, possible_anagrams: &[&'a str]) -> HashSet<&'a str> {
    //todo!("For the '{word}' word find anagrams among the following words: {possible_anagrams:?}");
    let chars = word.chars().collect::<HashSet<char>>();
    possible_anagrams.iter().filter_map(|&candidate| {
        if candidate.len() == word.len() && candidate.chars().collect::<HashSet<char>>() == chars {
            Some(candidate)
        } else {
            None
        }
    }).collect()
}

fn _3_anagrams_for<'a>(word: &str, possible_anagrams: &[&'a str]) -> HashSet<&'a str> {
    //todo!("For the '{word}' word find anagrams among the following words: {possible_anagrams:?}");
    let chars = word.chars().collect::<HashSet<char>>();
    possible_anagrams
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.len() == word.len() && candidate.chars().collect::<HashSet<char>>() == chars
        })
        .collect()
}

fn anagrams_for<'a>(word: &str, possible_anagrams: &[&'a str]) -> HashSet<&'a str> {
    //todo!("For the '{word}' word find anagrams among the following words: {possible_anagrams:?}");
    let wlc = word.to_lowercase();
    let mut chars : Vec<char> = wlc.chars().collect();
    chars.sort_unstable(); // Ensure the characters are sorted for comparison
    possible_anagrams
        .iter()
        .copied()
        .filter(|candidate| {
            let clc = candidate.to_lowercase();
            if clc == wlc {
                return false; // words_are_not_anagrams_of_themselves
            }
            let mut clc_chars = clc.chars().collect::<Vec<char>>();
            clc_chars.sort_unstable();
            clc_chars == chars
        })
        .collect()
}

// with itertools
#[cfg(FALSE)]
fn anagrams_for<'a>(word: &str, possible_anagrams: &[&'a str]) -> HashSet<&'a str> {
    //todo!("For the '{word}' word find anagrams among the following words: {possible_anagrams:?}");
    let wlc = word.to_lowercase();
    let chars : Vec<char> = wlc.chars().sorted().collect();
    possible_anagrams
        .iter()
        .copied()
        .filter(|candidate| {
            let clc = candidate.to_lowercase();
            wlc != clc && clc.chars().sorted().collect::<Vec<char>>() == chars
        })
        .collect()
}

fn _2_anagrams_for<'a>(word: &str, possible_anagrams: &[&'a str]) -> Vec<&'a str> {
    let chars = word.chars().collect::<HashSet<char>>();
    possible_anagrams.iter().filter(|&&candidate| {
        candidate.len() == word.len() && candidate.chars().collect::<HashSet<char>>() == chars
    }).copied().collect()
}

pub fn test_anagrams() {
    println!("Testing anagrams...");
    let word = "listen";
    let possible_anagrams = vec!["enlist", "google", "inlets", "banana"];
    let anagrams = anagrams_for(word, &possible_anagrams);
    assert!(anagrams.contains("enlist"));
    assert!(anagrams.contains("inlets"));
    assert!(!anagrams.contains("google"));
    assert!(!anagrams.contains("banana"));
}

fn select_words_by_len<'a>(words: &[&'a str], len: usize) -> Vec<&'a str> {
    words.iter().copied().filter(|word| word.len() == len).collect()
}

fn _2_select_words_by_len<'a>(words: &[&'a str], len: usize) -> Vec<&'a str> {
    words.iter().filter(|&&word| word.len() == len).copied().collect()
}

pub fn test_1() {
    println!("Testing select_words_by_len...");
    let words = vec!["hello", "world", "rust", "programming"];
    let filtered = select_words_by_len(&words, 5);
    assert_eq!(filtered, vec!["hello", "world"]);
}

#[test]
fn no_matches() {
let word = "diaper";
let inputs = &["hello", "world", "zombies", "pants"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_two_anagrams() {
let word = "solemn";
let inputs = &["lemons", "cherry", "melons"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["lemons", "melons"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn does_not_detect_anagram_subsets() {
let word = "good";
let inputs = &["dog", "goody"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_anagram() {
let word = "listen";
let inputs = &["enlists", "google", "inlets", "banana"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["inlets"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_three_anagrams() {
let word = "allergy";
let inputs = &[
"gallery",
"ballerina",
"regally",
"clergy",
"largely",
"leading",
];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["gallery", "regally", "largely"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_multiple_anagrams_with_different_case() {
let word = "nose";
let inputs = &["Eons", "ONES"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["Eons", "ONES"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn does_not_detect_non_anagrams_with_identical_checksum() {
let word = "mass";
let inputs = &["last"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_anagrams_case_insensitively() {
let word = "Orchestra";
let inputs = &["cashregister", "Carthorse", "radishes"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["Carthorse"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_anagrams_using_case_insensitive_subject() {
let word = "Orchestra";
let inputs = &["cashregister", "carthorse", "radishes"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["carthorse"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn detects_anagrams_using_case_insensitive_possible_matches() {
let word = "orchestra";
let inputs = &["cashregister", "Carthorse", "radishes"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["Carthorse"]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn does_not_detect_an_anagram_if_the_original_word_is_repeated() {
let word = "go";
let inputs = &["goGoGO"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn anagrams_must_use_all_letters_exactly_once() {
let word = "tapper";
let inputs = &["patter"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn words_are_not_anagrams_of_themselves() {
let word = "BANANA";
let inputs = &["BANANA"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn words_are_not_anagrams_of_themselves_even_if_letter_case_is_partially_different() {
let word = "BANANA";
let inputs = &["Banana"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn words_are_not_anagrams_of_themselves_even_if_letter_case_is_completely_different() {
let word = "BANANA";
let inputs = &["banana"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter([]);
assert_eq!(output, expected);
}
#[test]
#[ignore]
fn words_other_than_themselves_can_be_anagrams() {
let word = "LISTEN";
let inputs = &["LISTEN", "Silent"];
let output = anagrams_for(word, inputs);
let expected = HashSet::from_iter(["Silent"]);
assert_eq!(output, expected);
}