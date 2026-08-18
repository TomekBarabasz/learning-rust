mod anagram;
use anagram::{test_anagrams,test_1};

mod abbreviate;
use abbreviate::test_abbreviate;

fn main() {
    println!("Hello, world!");
    test_anagrams();
    test_1();
    test_abbreviate();
}
