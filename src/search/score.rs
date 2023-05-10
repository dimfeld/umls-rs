use stringmetrics::jaccard;

pub struct TrigramScorer<'a> {
    trigrams: Vec<&'a str>,
}

impl<'a> TrigramScorer<'a> {
    pub fn new(word: &str) -> TrigramScorer {
        TrigramScorer {
            trigrams: calculate_trigrams(word),
        }
    }

    pub fn score_trigrams(&self, other_trigrams: &[&str]) -> f32 {
        jaccard(self.trigrams.iter(), other_trigrams.iter())
    }

    pub fn score_word(&self, other_word: &str) -> f32 {
        let other_trigrams = calculate_trigrams(other_word);
        self.score_trigrams(&other_trigrams)
    }
}

fn calculate_trigrams(word: &str) -> Vec<&str> {
    let mut trigrams = Vec::with_capacity(word.len() + 2);
    trigrams.push(&word[0..1]);
    trigrams.push(&word[0..2]);
    for i in 0..word.len() - 2 {
        trigrams.push(&word[i..i + 3]);
    }

    if word.len() > 2 {
        trigrams.push(&word[word.len() - 2..]);
        if word.len() > 1 {
            trigrams.push(&word[word.len() - 1..]);
        }
    }

    trigrams
}
