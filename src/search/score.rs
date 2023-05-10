use stringmetrics::jaccard;

pub fn jaccard_trigram_distance(one: &str, two: &str) -> f32 {
    jaccard(TrigramIterator::new(one), TrigramIterator::new(two))
}

enum TrigramIteratorState {
    FirstOneGram,
    FirstTwoGram,
    Trigram(usize),
    LastTwoGram,
    LastOneGram,
    Done,
}

pub struct TrigramIterator<'a> {
    word: &'a str,
    state: TrigramIteratorState,
}

impl<'a> TrigramIterator<'a> {
    pub fn new(word: &str) -> TrigramIterator {
        TrigramIterator {
            word,
            state: TrigramIteratorState::FirstOneGram,
        }
    }
}

impl<'a> Iterator for TrigramIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (next_state, result) = match self.state {
            TrigramIteratorState::FirstOneGram => {
                let next_state = if self.word.len() > 1 {
                    TrigramIteratorState::FirstTwoGram
                } else {
                    TrigramIteratorState::Done
                };
                (next_state, &self.word[0..1])
            }
            TrigramIteratorState::FirstTwoGram => {
                let next_state = if self.word.len() > 2 {
                    TrigramIteratorState::Trigram(0)
                } else {
                    TrigramIteratorState::LastOneGram
                };
                (next_state, &self.word[0..2])
            }
            TrigramIteratorState::Trigram(index) => {
                let next_state = if index + 3 >= self.word.len() {
                    TrigramIteratorState::LastTwoGram
                } else {
                    TrigramIteratorState::Trigram(index + 1)
                };

                (next_state, &self.word[index..index + 3])
            }

            TrigramIteratorState::LastTwoGram => (
                TrigramIteratorState::LastOneGram,
                &self.word[self.word.len() - 2..],
            ),
            TrigramIteratorState::LastOneGram => (
                TrigramIteratorState::Done,
                &self.word[self.word.len() - 1..],
            ),
            TrigramIteratorState::Done => return None,
        };

        self.state = next_state;
        Some(result)
    }
}

#[cfg(test)]
mod test {
    use super::TrigramIterator;

    #[test]
    fn trigram_iterator() {
        let result = TrigramIterator::new("abcdef").collect::<Vec<_>>();
        assert_eq!(
            result,
            vec!["a", "ab", "abc", "bcd", "cde", "def", "ef", "f"]
        );
    }

    #[test]
    fn empty() {
        let result = TrigramIterator::new("a").collect::<Vec<_>>();
        assert_eq!(result, vec!["a"]);
    }

    #[test]
    fn one_letter_word() {
        let result = TrigramIterator::new("a").collect::<Vec<_>>();
        assert_eq!(result, vec!["a"]);
    }

    #[test]
    fn two_letter_word() {
        let result = TrigramIterator::new("ab").collect::<Vec<_>>();
        assert_eq!(result, vec!["a", "ab", "b"]);
    }

    #[test]
    fn three_letter_word() {
        let result = TrigramIterator::new("abc").collect::<Vec<_>>();
        assert_eq!(result, vec!["a", "ab", "abc", "bc", "c"]);
    }

    #[test]
    fn four_letter_word() {
        let result = TrigramIterator::new("abcd").collect::<Vec<_>>();
        assert_eq!(result, vec!["a", "ab", "abc", "bcd", "cd", "d"]);
    }
}
