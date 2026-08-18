use std::collections::HashMap;

fn is_valid(n : char) -> bool {
    matches!(n, 'A' | 'C' | 'G' | 'T')
}
fn is_valid_nucleotide(nucleotide: char) -> bool {
    "ACGT".contains(nucleotide)
}
pub fn count(nucleotide: char, dna: &str) -> Result<usize, char> {
    if !is_valid(nucleotide) {
        return Err(nucleotide);
    }
    dna.chars().try_fold(0, |acc, n| {
        if !is_valid(n) {
            Err(n)
        } else if n == nucleotide {
            Ok(acc + 1)
        } else {
            Ok(acc)
        }
    })
}

pub fn nucleotide_counts_1(dna: &str) -> Result<HashMap<char, usize>, char> {
    dna.chars().try_fold(HashMap::new(),|mut acc,n| {
        if !"ACGT".contains(n) {
            Err(n)
        } else {
            *acc.entry(n).or_insert(0) += 1;
            Ok(acc)
        }
    })
}
pub fn nucleotide_counts(dna: &str) -> Result<HashMap<char, usize>, char> {
    dna.chars().try_fold(
        HashMap::from([('A', 0),('C', 0),('G', 0),('T', 0)]),
        |mut acc,n| {
            if !"ACGT".contains(n) /*or !acc.contains_key(&n)*/{
                Err(n)
            } else {
                acc.entry(n).and_modify(|cnt| *cnt += 1);
                Ok(acc)
            }
        }
    )
}