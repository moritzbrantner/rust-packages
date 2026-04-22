use std::collections::BTreeSet;

use text_analysis_core::{text_stats, tokenize_words, word_counts, TextStats};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct TermFrequency {
    pub term: String,
    pub count: usize,
    pub frequency: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextFeatureSummary {
    pub stats: TextStats,
    pub unique_terms: usize,
    pub lexical_diversity: f32,
    pub top_terms: Vec<TermFrequency>,
}

pub fn summarize_text(text: &str, max_terms: usize) -> TextFeatureSummary {
    let stats = text_stats(text);
    let top_terms = top_terms(text, max_terms, &BTreeSet::new());
    let unique_terms = word_counts(text).len();
    let lexical_diversity = if stats.words == 0 {
        0.0
    } else {
        unique_terms as f32 / stats.words as f32
    };
    TextFeatureSummary {
        stats,
        unique_terms,
        lexical_diversity,
        top_terms,
    }
}

pub fn term_frequencies(text: &str) -> Vec<TermFrequency> {
    let counts = word_counts(text);
    let total = counts.values().sum::<usize>().max(1) as f32;
    let mut terms = counts
        .into_iter()
        .map(|(term, count)| TermFrequency {
            term,
            count,
            frequency: count as f32 / total,
        })
        .collect::<Vec<_>>();
    terms.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.term.cmp(&right.term))
    });
    terms
}

pub fn top_terms(text: &str, limit: usize, stop_words: &BTreeSet<String>) -> Vec<TermFrequency> {
    term_frequencies(text)
        .into_iter()
        .filter(|term| !stop_words.contains(&term.term))
        .take(limit)
        .collect()
}

pub fn character_ngrams(text: &str, n: usize) -> Result<Vec<String>> {
    if n == 0 {
        return Err(DetectError::InvalidArgument(
            "ngram size must be greater than zero".to_string(),
        ));
    }
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() < n {
        return Ok(Vec::new());
    }
    Ok(chars
        .windows(n)
        .map(|window| window.iter().collect::<String>())
        .collect())
}

pub fn token_ngrams(text: &str, n: usize) -> Result<Vec<Vec<String>>> {
    if n == 0 {
        return Err(DetectError::InvalidArgument(
            "ngram size must be greater than zero".to_string(),
        ));
    }
    let tokens = tokenize_words(text);
    if tokens.len() < n {
        return Ok(Vec::new());
    }
    Ok(tokens.windows(n).map(|window| window.to_vec()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_terms_by_count() {
        let terms = term_frequencies("red blue red green blue red");
        assert_eq!(terms[0].term, "red");
        assert_eq!(terms[0].count, 3);
    }

    #[test]
    fn builds_token_ngrams() {
        let ngrams = token_ngrams("a b c", 2).unwrap();
        assert_eq!(
            ngrams,
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["b".to_string(), "c".to_string()]
            ]
        );
    }
}
