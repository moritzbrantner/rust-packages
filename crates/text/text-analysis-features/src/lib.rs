use std::collections::{BTreeMap, BTreeSet};

use text_analysis_core::{
    detailed_text_stats, text_stats, tokenize, tokenize_words, word_counts, TextProcessingOptions,
    TextStats, TokenKind,
};
use video_analysis_core::{
    AnalysisEvent, DetectError, Result, TextAnalyzer, TextSegment, Timestamp,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopWords {
    pub language: Option<String>,
    pub terms: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordOptions {
    pub max_terms: usize,
    pub min_term_len: usize,
    pub stop_words: StopWords,
}

impl Default for KeywordOptions {
    fn default() -> Self {
        Self {
            max_terms: 10,
            min_term_len: 3,
            stop_words: english_stop_words(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keyword {
    pub text: String,
    pub score: f32,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NgramFrequency {
    pub terms: Vec<String>,
    pub count: usize,
    pub frequency: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadabilitySummary {
    pub sentence_count: usize,
    pub word_count: usize,
    pub average_sentence_words: f32,
    pub average_word_chars: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextAnalysisOptions {
    pub processing: TextProcessingOptions,
    pub keywords: KeywordOptions,
    pub emit_top_terms: usize,
    pub emit_patterns: bool,
}

impl Default for TextAnalysisOptions {
    fn default() -> Self {
        Self {
            processing: TextProcessingOptions::default(),
            keywords: KeywordOptions::default(),
            emit_top_terms: 5,
            emit_patterns: true,
        }
    }
}

pub fn summarize_text(text: &str, max_terms: usize) -> TextFeatureSummary {
    let stats = text_stats(text);
    let stop_words = english_stop_words();
    let top_terms = top_terms(text, max_terms, &stop_words.terms);
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

pub fn english_stop_words() -> StopWords {
    let terms = [
        "a", "about", "after", "all", "also", "an", "and", "any", "are", "as", "at", "be",
        "because", "been", "but", "by", "can", "do", "for", "from", "had", "has", "have", "he",
        "her", "his", "how", "i", "if", "in", "into", "is", "it", "its", "just", "me", "more",
        "my", "no", "not", "of", "on", "or", "our", "out", "she", "so", "some", "than", "that",
        "the", "their", "them", "then", "there", "these", "they", "this", "to", "up", "us", "was",
        "we", "were", "what", "when", "which", "who", "will", "with", "would", "you", "your",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    StopWords {
        language: Some("en".to_string()),
        terms,
    }
}

pub fn keywords(text: &str, options: &KeywordOptions) -> Vec<Keyword> {
    let processing = TextProcessingOptions::default();
    let mut counts = BTreeMap::<String, usize>::new();
    for token in tokenize(text, &processing) {
        if !matches!(
            token.kind,
            TokenKind::Word | TokenKind::Number | TokenKind::Email | TokenKind::Url
        ) {
            continue;
        }
        if token.normalized.chars().count() < options.min_term_len {
            continue;
        }
        if options.stop_words.terms.contains(&token.normalized) {
            continue;
        }
        *counts.entry(token.normalized).or_insert(0) += 1;
    }

    let total = counts.values().sum::<usize>().max(1) as f32;
    let mut terms = counts
        .into_iter()
        .map(|(text, count)| Keyword {
            text,
            score: count as f32 / total,
            count,
        })
        .collect::<Vec<_>>();
    terms.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.text.cmp(&right.text))
    });
    terms.truncate(options.max_terms);
    terms
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

pub fn character_ngram_frequencies(text: &str, n: usize) -> Result<Vec<NgramFrequency>> {
    let ngrams = character_ngrams(text, n)?;
    Ok(ngram_frequencies(
        ngrams
            .into_iter()
            .map(|ngram| vec![ngram])
            .collect::<Vec<_>>(),
    ))
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

pub fn token_ngram_frequencies(
    text: &str,
    n: usize,
    options: &TextProcessingOptions,
) -> Result<Vec<NgramFrequency>> {
    if n == 0 {
        return Err(DetectError::InvalidArgument(
            "ngram size must be greater than zero".to_string(),
        ));
    }
    let tokens = tokenize(text, options)
        .into_iter()
        .filter(|token| token.kind != TokenKind::Punctuation)
        .map(|token| token.normalized)
        .collect::<Vec<_>>();
    if tokens.len() < n {
        return Ok(Vec::new());
    }
    let ngrams = tokens
        .windows(n)
        .map(|window| window.to_vec())
        .collect::<Vec<_>>();
    Ok(ngram_frequencies(ngrams))
}

pub fn readability_summary(text: &str, options: &TextProcessingOptions) -> ReadabilitySummary {
    let stats = detailed_text_stats(text, options);
    ReadabilitySummary {
        sentence_count: stats.basic.sentences,
        word_count: stats.basic.words,
        average_sentence_words: stats.average_words_per_sentence,
        average_word_chars: stats.average_chars_per_word,
    }
}

#[derive(Debug, Default, Clone)]
pub struct TextStatsAnalyzer;

impl TextAnalyzer for TextStatsAnalyzer {
    fn name(&self) -> &str {
        "text_stats"
    }

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
        let stats = text_stats(segment.text);
        Ok(vec![
            event_at(self.name(), "text:stats", segment.timestamp).score(stats.words as f32)
        ])
    }
}

#[derive(Debug, Clone)]
pub struct KeywordAnalyzer {
    pub options: KeywordOptions,
}

impl Default for KeywordAnalyzer {
    fn default() -> Self {
        Self {
            options: KeywordOptions::default(),
        }
    }
}

impl KeywordAnalyzer {
    pub fn new(options: KeywordOptions) -> Self {
        Self { options }
    }
}

impl TextAnalyzer for KeywordAnalyzer {
    fn name(&self) -> &str {
        "keywords"
    }

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
        Ok(keywords(segment.text, &self.options)
            .into_iter()
            .map(|keyword| {
                event_at(
                    self.name(),
                    &format!("text:keyword:{}", keyword.text),
                    segment.timestamp,
                )
                .score(keyword.score)
            })
            .collect())
    }
}

#[derive(Debug, Default, Clone)]
pub struct PatternAnalyzer;

impl TextAnalyzer for PatternAnalyzer {
    fn name(&self) -> &str {
        "text_patterns"
    }

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
        let mut events = pattern_events(self.name(), segment.text, segment.timestamp);
        if segment.text.trim_end().ends_with(['?', '؟', '？']) {
            events.push(event_at(
                self.name(),
                "text:pattern:question",
                segment.timestamp,
            ));
        }
        Ok(events)
    }
}

#[derive(Debug, Default, Clone)]
pub struct TranscriptHeuristicAnalyzer;

impl TextAnalyzer for TranscriptHeuristicAnalyzer {
    fn name(&self) -> &str {
        "transcript_heuristics"
    }

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
        let mut events = Vec::new();
        let text = segment.text.trim();
        if text.ends_with(['?', '؟', '？']) {
            events.push(event_at(self.name(), "speech:question", segment.timestamp));
        }
        if has_token_kind(text, TokenKind::Url) {
            events.push(event_at(self.name(), "speech:url", segment.timestamp));
        }
        if has_token_kind(text, TokenKind::Number) {
            events.push(event_at(self.name(), "speech:number", segment.timestamp));
        }
        if tokenize_words(text).len() >= 30 {
            events.push(event_at(
                self.name(),
                "speech:long_segment",
                segment.timestamp,
            ));
        }
        Ok(events)
    }
}

fn ngram_frequencies(ngrams: Vec<Vec<String>>) -> Vec<NgramFrequency> {
    let total = ngrams.len().max(1) as f32;
    let mut counts = BTreeMap::<Vec<String>, usize>::new();
    for ngram in ngrams {
        *counts.entry(ngram).or_insert(0) += 1;
    }
    let mut frequencies = counts
        .into_iter()
        .map(|(terms, count)| NgramFrequency {
            terms,
            count,
            frequency: count as f32 / total,
        })
        .collect::<Vec<_>>();
    frequencies.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.terms.cmp(&right.terms))
    });
    frequencies
}

fn pattern_events(analyzer: &str, text: &str, timestamp: Option<Timestamp>) -> Vec<AnalysisEvent> {
    let mut seen = BTreeSet::new();
    let mut events = Vec::new();
    for token in tokenize(text, &TextProcessingOptions::default()) {
        let label = match token.kind {
            TokenKind::Url => Some("text:pattern:url"),
            TokenKind::Email => Some("text:pattern:email"),
            TokenKind::Mention => Some("text:pattern:mention"),
            TokenKind::Hashtag => Some("text:pattern:hashtag"),
            TokenKind::Number => Some("text:pattern:number"),
            _ => None,
        };
        if let Some(label) = label {
            if seen.insert(label) {
                events.push(event_at(analyzer, label, timestamp));
            }
        }
    }
    events
}

fn has_token_kind(text: &str, kind: TokenKind) -> bool {
    tokenize(text, &TextProcessingOptions::default())
        .into_iter()
        .any(|token| token.kind == kind)
}

fn event_at(analyzer: &str, label: &str, timestamp: Option<Timestamp>) -> AnalysisEvent {
    let event = AnalysisEvent::new(analyzer, label);
    if let Some(timestamp) = timestamp {
        event.at_timestamp(timestamp)
    } else {
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_core::{OwnedTextSegment, TextPipeline};

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

    #[test]
    fn filters_english_stop_words_for_keywords() {
        let terms = keywords(
            "the rust code and the rust tests",
            &KeywordOptions::default(),
        );
        assert_eq!(terms[0].text, "rust");
        assert!(!terms.iter().any(|term| term.text == "the"));
    }

    #[test]
    fn counts_token_ngram_frequencies() {
        let ngrams =
            token_ngram_frequencies("red blue red blue green", 2, &Default::default()).unwrap();
        assert_eq!(ngrams[0].terms, vec!["red", "blue"]);
        assert_eq!(ngrams[0].count, 2);
    }

    #[test]
    fn counts_character_ngram_frequencies() {
        let ngrams = character_ngram_frequencies("ababa", 2).unwrap();
        assert_eq!(ngrams[0].terms, vec!["ab"]);
        assert_eq!(ngrams[0].count, 2);
    }

    #[test]
    fn computes_readability_summary() {
        let summary = readability_summary("One sentence. Two words here.", &Default::default());
        assert_eq!(summary.sentence_count, 2);
        assert_eq!(summary.word_count, 5);
    }

    #[test]
    fn pattern_analyzer_emits_labels() {
        let mut analyzer = PatternAnalyzer;
        let segment =
            OwnedTextSegment::new(0, "Mail hi@example.com @team #rust https://example.com 42?");
        let labels = analyzer
            .process_segment(&segment.as_segment())
            .unwrap()
            .into_iter()
            .map(|event| event.label)
            .collect::<Vec<_>>();
        assert!(labels.contains(&"text:pattern:url".to_string()));
        assert!(labels.contains(&"text:pattern:number".to_string()));
        assert!(labels.contains(&"text:pattern:question".to_string()));
        assert!(labels.contains(&"text:pattern:email".to_string()));
        assert!(labels.contains(&"text:pattern:mention".to_string()));
        assert!(labels.contains(&"text:pattern:hashtag".to_string()));
    }

    #[test]
    fn analyzers_run_inside_text_pipeline() {
        let mut pipeline = TextPipeline::builder()
            .analyzer(TextStatsAnalyzer)
            .analyzer(KeywordAnalyzer::default())
            .analyzer(TranscriptHeuristicAnalyzer)
            .build()
            .unwrap();

        pipeline
            .process_segment(OwnedTextSegment::new(
                0,
                "Visit https://example.com with rust rust?",
            ))
            .unwrap();
        let result = pipeline.finish_analysis().unwrap();
        let labels = result
            .events
            .into_iter()
            .map(|event| event.label)
            .collect::<Vec<_>>();
        assert!(labels.contains(&"text:stats".to_string()));
        assert!(labels.contains(&"text:keyword:rust".to_string()));
        assert!(labels.contains(&"speech:question".to_string()));
        assert!(labels.contains(&"speech:url".to_string()));
    }
}
