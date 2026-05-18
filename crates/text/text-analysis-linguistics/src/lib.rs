#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use text_analysis_core::{
    build_annotation_graph_from_parts, detect_script_profile, split_paragraphs,
    split_sentence_spans, tokenize, tokenize_words, AnnotationConfidence, AnnotationProvenance,
    Sentence, TextAnnotationGraph, TextDocument, TextProcessingOptions, TextSpan, Token, TokenKind,
};
use text_analysis_models::{TokenizedText, TokenizerBundle, TokenizerSource};
use text_analysis_transcription::{TranscriptSegment, TranscriptionResult};
use video_analysis_core::{AnalysisEvent, OwnedTextSegment, Result, TextAnalyzer, TextSegment};

#[derive(Debug, Clone, PartialEq)]
/// Data type for language prediction.
pub struct LanguagePrediction {
    /// Language tag for this value.
    pub language: String,
    /// Confidence score for this value.
    pub confidence: f32,
    /// The script value.
    pub script: Option<String>,
    /// The reason value.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for language profile.
pub struct LanguageProfile {
    /// The primary value.
    pub primary: Option<LanguagePrediction>,
    /// The alternatives value.
    pub alternatives: Vec<LanguagePrediction>,
    /// The dominant script value.
    pub dominant_script: Option<String>,
    /// The is mixed value.
    pub is_mixed: bool,
    /// The sentence predictions value.
    pub sentence_predictions: Vec<Option<LanguagePrediction>>,
    /// The token count value.
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for language detection options.
pub struct LanguageDetectionOptions {
    /// The min tokens for decision value.
    pub min_tokens_for_decision: usize,
    /// The max alternatives value.
    pub max_alternatives: usize,
    /// The mixed threshold value.
    pub mixed_threshold: f32,
    /// The sentence level value.
    pub sentence_level: bool,
}

impl Default for LanguageDetectionOptions {
    fn default() -> Self {
        Self {
            min_tokens_for_decision: 2,
            max_alternatives: 3,
            mixed_threshold: 0.12,
            sentence_level: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for language lexicon.
pub struct LanguageLexicon<T> {
    /// Language tag for this value.
    pub language: String,
    /// The entries value.
    pub entries: BTreeSet<T>,
}

impl<T: Ord> LanguageLexicon<T> {
    /// Creates a new value.
    pub fn new(language: impl Into<String>, entries: impl IntoIterator<Item = T>) -> Self {
        Self {
            language: language.into(),
            entries: entries.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for lexicon entry.
pub struct LexiconEntry {
    /// The term value.
    pub term: String,
    /// The category value.
    pub category: String,
    /// The weight value.
    pub weight: f32,
}

impl LexiconEntry {
    /// Creates a new value.
    pub fn new(term: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            term: term.into(),
            category: category.into(),
            weight: 1.0,
        }
    }

    /// Returns weight.
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for lexicon store.
pub struct LexiconStore {
    /// The stop words value.
    pub stop_words: BTreeMap<String, LanguageLexicon<String>>,
    /// The abbreviations value.
    pub abbreviations: BTreeMap<String, LanguageLexicon<String>>,
    /// The lexical classes value.
    pub lexical_classes: BTreeMap<String, Vec<LexiconEntry>>,
    /// The gazetteers value.
    pub gazetteers: BTreeMap<String, Vec<LexiconEntry>>,
    /// The valency hints value.
    pub valency_hints: BTreeMap<String, Vec<LexiconEntry>>,
    /// The sentiment terms value.
    pub sentiment_terms: BTreeMap<String, Vec<LexiconEntry>>,
}

impl Default for LexiconStore {
    fn default() -> Self {
        Self::multilingual_defaults()
    }
}

impl LexiconStore {
    /// Returns multilingual defaults.
    pub fn multilingual_defaults() -> Self {
        let stop_words = BTreeMap::from([
            (
                "en".to_string(),
                LanguageLexicon::new(
                    "en",
                    [
                        "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "from",
                        "has", "have", "he", "i", "in", "is", "it", "of", "on", "or", "she",
                        "that", "the", "they", "this", "to", "was", "we", "were", "with", "you",
                    ]
                    .into_iter()
                    .map(String::from),
                ),
            ),
            (
                "de".to_string(),
                LanguageLexicon::new(
                    "de",
                    [
                        "aber", "als", "am", "auch", "auf", "bei", "das", "der", "die", "ein",
                        "eine", "er", "es", "für", "ich", "im", "in", "ist", "mit", "nicht", "sie",
                        "und", "von", "wir", "zu",
                    ]
                    .into_iter()
                    .map(String::from),
                ),
            ),
            (
                "es".to_string(),
                LanguageLexicon::new(
                    "es",
                    [
                        "a", "con", "de", "del", "el", "ella", "en", "es", "la", "las", "lo",
                        "los", "para", "por", "que", "se", "su", "un", "una", "y", "yo",
                    ]
                    .into_iter()
                    .map(String::from),
                ),
            ),
            (
                "fr".to_string(),
                LanguageLexicon::new(
                    "fr",
                    [
                        "à", "avec", "ce", "de", "des", "du", "elle", "en", "est", "et", "il",
                        "je", "la", "le", "les", "mais", "nous", "pas", "pour", "que", "qui", "un",
                        "une", "vous",
                    ]
                    .into_iter()
                    .map(String::from),
                ),
            ),
        ]);

        let abbreviations = BTreeMap::from([(
            "en".to_string(),
            LanguageLexicon::new(
                "en",
                [
                    "mr.", "mrs.", "ms.", "dr.", "prof.", "inc.", "corp.", "ltd.",
                ]
                .into_iter()
                .map(String::from),
            ),
        )]);

        let lexical_classes = BTreeMap::from([
            (
                "discourse_markers".to_string(),
                vec![
                    LexiconEntry::new("however", "contrast"),
                    LexiconEntry::new("therefore", "cause"),
                    LexiconEntry::new("because", "cause"),
                    LexiconEntry::new("first", "transition"),
                    LexiconEntry::new("finally", "transition"),
                ],
            ),
            (
                "disfluencies".to_string(),
                vec![
                    LexiconEntry::new("uh", "disfluency"),
                    LexiconEntry::new("um", "disfluency"),
                    LexiconEntry::new("er", "disfluency"),
                    LexiconEntry::new("ah", "disfluency"),
                    LexiconEntry::new("hmm", "disfluency"),
                ],
            ),
        ]);

        let gazetteers = BTreeMap::from([
            (
                "months".to_string(),
                [
                    "january",
                    "february",
                    "march",
                    "april",
                    "may",
                    "june",
                    "july",
                    "august",
                    "september",
                    "october",
                    "november",
                    "december",
                ]
                .into_iter()
                .map(|month| LexiconEntry::new(month, "month"))
                .collect(),
            ),
            (
                "org_suffixes".to_string(),
                ["inc", "corp", "ltd", "llc", "gmbh", "ag"]
                    .into_iter()
                    .map(|suffix| LexiconEntry::new(suffix, "organization_suffix"))
                    .collect(),
            ),
        ]);

        let valency_hints = BTreeMap::from([(
            "verbs".to_string(),
            vec![
                LexiconEntry::new("say", "attribution"),
                LexiconEntry::new("announce", "attribution"),
                LexiconEntry::new("launch", "action"),
                LexiconEntry::new("visit", "movement"),
                LexiconEntry::new("present", "communication"),
            ],
        )]);

        let sentiment_terms = BTreeMap::from([(
            "en".to_string(),
            vec![
                LexiconEntry::new("excellent", "positive").weight(1.5),
                LexiconEntry::new("reliable", "positive").weight(1.0),
                LexiconEntry::new("terrible", "negative").weight(2.0),
                LexiconEntry::new("broken", "negative").weight(1.5),
            ],
        )]);

        Self {
            stop_words,
            abbreviations,
            lexical_classes,
            gazetteers,
            valency_hints,
            sentiment_terms,
        }
    }

    /// Returns stop words for.
    pub fn stop_words_for(&self, language: &str) -> Option<&LanguageLexicon<String>> {
        self.stop_words.get(language)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
/// Data type for language detector.
pub struct LanguageDetector {
    /// The options value.
    pub options: LanguageDetectionOptions,
    /// The lexicons value.
    pub lexicons: LexiconStore,
}

impl LanguageDetector {
    /// Returns detect text.
    pub fn detect_text(&self, text: &str) -> LanguageProfile {
        let word_tokens = tokenize_words(text);
        let token_count = word_tokens.len();
        let script_profile = detect_script_profile(text);
        let dominant_script = script_profile.dominant_script.clone();

        let mut scored = self.language_scores(&word_tokens, dominant_script.as_deref());
        scored.sort_by(|left, right| {
            right
                .confidence
                .total_cmp(&left.confidence)
                .then_with(|| left.language.cmp(&right.language))
        });
        if scored.is_empty() {
            scored.push(LanguagePrediction {
                language: dominant_script
                    .as_deref()
                    .and_then(script_default_language)
                    .unwrap_or("und")
                    .to_string(),
                confidence: if token_count == 0 { 0.0 } else { 0.2 },
                script: dominant_script.clone(),
                reason: "script fallback".to_string(),
            });
        }

        let primary = scored.first().cloned();
        let alternatives = scored
            .iter()
            .skip(1)
            .take(self.options.max_alternatives)
            .cloned()
            .collect::<Vec<_>>();
        let is_mixed = primary
            .as_ref()
            .zip(alternatives.first())
            .map(|(first, second)| {
                (first.confidence - second.confidence).abs() <= self.options.mixed_threshold
            })
            .unwrap_or(script_profile.is_mixed)
            || script_profile.is_mixed;

        let sentence_predictions = if self.options.sentence_level {
            split_sentence_spans(text, &TextProcessingOptions::default())
                .into_iter()
                .map(|sentence| {
                    self.language_scores(
                        &tokenize_words(&sentence.text),
                        dominant_script.as_deref(),
                    )
                    .into_iter()
                    .max_by(|left, right| left.confidence.total_cmp(&right.confidence))
                })
                .collect()
        } else {
            Vec::new()
        };

        LanguageProfile {
            primary,
            alternatives,
            dominant_script,
            is_mixed,
            sentence_predictions,
            token_count,
        }
    }

    fn language_scores(
        &self,
        tokens: &[String],
        dominant_script: Option<&str>,
    ) -> Vec<LanguagePrediction> {
        let mut predictions = Vec::new();
        let lower_tokens = tokens
            .iter()
            .map(|token| token.to_lowercase())
            .collect::<Vec<_>>();
        for (language, lexicon) in &self.lexicons.stop_words {
            let hits = lower_tokens
                .iter()
                .filter(|token| lexicon.entries.contains(*token))
                .count();
            let lexical_score = if lower_tokens.is_empty() {
                0.0
            } else {
                hits as f32 / lower_tokens.len() as f32
            };
            let script_bonus = dominant_script
                .and_then(script_default_language)
                .map(|candidate| if candidate == language { 0.15 } else { 0.0 })
                .unwrap_or_default();
            let confidence = (lexical_score + script_bonus).clamp(0.0, 1.0);
            if confidence > 0.0 {
                predictions.push(LanguagePrediction {
                    language: language.clone(),
                    confidence,
                    script: dominant_script.map(ToString::to_string),
                    reason: if hits > 0 {
                        format!("matched {hits} stop words")
                    } else {
                        "script fallback".to_string()
                    },
                });
            }
        }
        if predictions.is_empty() {
            if let Some(language) = dominant_script.and_then(script_default_language) {
                predictions.push(LanguagePrediction {
                    language: language.to_string(),
                    confidence: 0.35,
                    script: dominant_script.map(ToString::to_string),
                    reason: "script fallback".to_string(),
                });
            }
        }
        predictions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing tokenization mode.
pub enum TokenizationMode {
    /// The word variant.
    Word,
    /// The subword variant.
    Subword,
    /// The mixed variant.
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenizer policy.
pub struct TokenizerPolicy {
    /// The mode value.
    pub mode: TokenizationMode,
    /// The default source value.
    pub default_source: TokenizerSource,
    /// The language overrides value.
    pub language_overrides: BTreeMap<String, TokenizerSource>,
    /// The task overrides value.
    pub task_overrides: BTreeMap<String, TokenizerSource>,
    /// The model family overrides value.
    pub model_family_overrides: BTreeMap<String, TokenizerSource>,
}

impl Default for TokenizerPolicy {
    fn default() -> Self {
        Self {
            mode: TokenizationMode::Mixed,
            default_source: TokenizerSource::default(),
            language_overrides: BTreeMap::new(),
            task_overrides: BTreeMap::new(),
            model_family_overrides: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for tokenizer selection.
pub struct TokenizerSelection {
    /// The mode value.
    pub mode: TokenizationMode,
    /// The source value.
    pub source: Option<TokenizerSource>,
    /// Language tag for this value.
    pub language: Option<String>,
    /// The task value.
    pub task: Option<String>,
    /// The model family value.
    pub model_family: Option<String>,
    /// The reason value.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for tokenizer registry.
pub struct TokenizerRegistry {
    /// The policy value.
    pub policy: TokenizerPolicy,
}

impl TokenizerRegistry {
    /// Returns select.
    pub fn select(
        &self,
        language: Option<&str>,
        task: Option<&str>,
        model_family: Option<&str>,
    ) -> TokenizerSelection {
        let source = match self.policy.mode {
            TokenizationMode::Word => None,
            TokenizationMode::Subword | TokenizationMode::Mixed => Some(
                task.and_then(|task| self.policy.task_overrides.get(task))
                    .or_else(|| {
                        model_family
                            .and_then(|family| self.policy.model_family_overrides.get(family))
                    })
                    .or_else(|| {
                        language.and_then(|language| self.policy.language_overrides.get(language))
                    })
                    .cloned()
                    .unwrap_or_else(|| self.policy.default_source.clone()),
            ),
        };
        let reason = if let Some(task) =
            task.filter(|task| self.policy.task_overrides.contains_key(*task))
        {
            format!("task override for `{task}`")
        } else if let Some(family) =
            model_family.filter(|family| self.policy.model_family_overrides.contains_key(*family))
        {
            format!("model family override for `{family}`")
        } else if let Some(language) =
            language.filter(|language| self.policy.language_overrides.contains_key(*language))
        {
            format!("language override for `{language}`")
        } else {
            "default tokenizer policy".to_string()
        };
        TokenizerSelection {
            mode: self.policy.mode,
            source,
            language: language.map(ToString::to_string),
            task: task.map(ToString::to_string),
            model_family: model_family.map(ToString::to_string),
            reason,
        }
    }

    /// Returns align.
    pub fn align(
        &self,
        text: &str,
        tokens: &[Token],
        selection: &TokenizerSelection,
    ) -> Result<Option<TokenAlignmentMap>> {
        let Some(source) = selection.source.clone() else {
            return Ok(None);
        };
        let bundle = TokenizerBundle::from_cached_source(source)?;
        let tokenized = bundle.tokenize(text)?;
        Ok(Some(align_tokenized_text(
            text,
            tokens,
            selection.clone(),
            &tokenized,
        )?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for subword span.
pub struct SubwordSpan {
    /// The index value.
    pub index: usize,
    /// The input identifier value.
    pub input_id: i64,
    /// The span value.
    pub span: Option<TextSpan>,
    /// Text content for this value.
    pub text: Option<String>,
    /// The token type identifier value.
    pub token_type_id: Option<i64>,
    /// The attention value.
    pub attention: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for aligned token.
pub struct AlignedToken {
    /// The token index value.
    pub token_index: usize,
    /// The token value.
    pub token: Token,
    /// The subword indices value.
    pub subword_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for token alignment map.
pub struct TokenAlignmentMap {
    /// The selection value.
    pub selection: TokenizerSelection,
    /// The subwords value.
    pub subwords: Vec<SubwordSpan>,
    /// The aligned tokens value.
    pub aligned_tokens: Vec<AlignedToken>,
}

/// Returns align tokenized text.
pub fn align_tokenized_text(
    text: &str,
    tokens: &[Token],
    selection: TokenizerSelection,
    tokenized: &TokenizedText,
) -> Result<TokenAlignmentMap> {
    let subwords = tokenized
        .input_ids
        .iter()
        .enumerate()
        .map(|(index, input_id)| {
            let span = tokenized
                .offsets
                .get(index)
                .copied()
                .flatten()
                .and_then(|(start, end)| {
                    if start >= end {
                        None
                    } else {
                        byte_span_to_text_span(text, start, end)
                    }
                });
            let subword_text = span
                .map(|span| text[span.byte_start..span.byte_end].to_string())
                .filter(|value| !value.is_empty());
            SubwordSpan {
                index,
                input_id: *input_id,
                span,
                text: subword_text,
                token_type_id: tokenized
                    .token_type_ids
                    .as_ref()
                    .and_then(|values| values.get(index).copied()),
                attention: tokenized
                    .attention_mask
                    .get(index)
                    .copied()
                    .unwrap_or_default()
                    != 0,
            }
        })
        .collect::<Vec<_>>();

    let aligned_tokens = tokens
        .iter()
        .cloned()
        .enumerate()
        .map(|(token_index, token)| {
            let subword_indices = subwords
                .iter()
                .filter_map(|subword| {
                    let span = subword.span?;
                    spans_overlap(token.span, span).then_some(subword.index)
                })
                .collect();
            AlignedToken {
                token_index,
                token,
                subword_indices,
            }
        })
        .collect();

    Ok(TokenAlignmentMap {
        selection,
        subwords,
        aligned_tokens,
    })
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for lemma.
pub struct Lemma {
    /// The token index value.
    pub token_index: usize,
    /// The value value.
    pub value: String,
    /// Language tag for this value.
    pub language: Option<String>,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// Data type for lemma options.
pub struct LemmaOptions {
    /// Language tag for this value.
    pub language: Option<String>,
    /// The lowercase proper nouns value.
    pub lowercase_proper_nouns: bool,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for lemmatization result.
pub struct LemmatizationResult {
    /// Language tag for this value.
    pub language: Option<String>,
    /// The lemmas value.
    pub lemmas: Vec<Lemma>,
}

/// Returns lemmatize tokens.
pub fn lemmatize_tokens(
    tokens: &[Token],
    language: Option<&str>,
    options: &LemmaOptions,
) -> LemmatizationResult {
    let language = options
        .language
        .as_deref()
        .or(language)
        .map(ToString::to_string);
    let lemmas = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            let (value, confidence) = heuristic_lemma(token, language.as_deref(), options);
            Lemma {
                token_index: index,
                value,
                language: language.clone(),
                confidence,
            }
        })
        .collect();
    LemmatizationResult { language, lemmas }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Variants describing morph feature.
pub enum MorphFeature {
    /// The number sing variant.
    NumberSing,
    /// The number plur variant.
    NumberPlur,
    /// The person1 variant.
    Person1,
    /// The person2 variant.
    Person2,
    /// The person3 variant.
    Person3,
    /// The tense past variant.
    TensePast,
    /// The tense pres variant.
    TensePres,
    /// The tense fut variant.
    TenseFut,
    /// The aspect prog variant.
    AspectProg,
    /// The aspect perf variant.
    AspectPerf,
    /// The mood imp variant.
    MoodImp,
    /// The verb form fin variant.
    VerbFormFin,
    /// The verb form inf variant.
    VerbFormInf,
    /// The case nom variant.
    CaseNom,
    /// The case acc variant.
    CaseAcc,
    /// The case dat variant.
    CaseDat,
    /// The gender masc variant.
    GenderMasc,
    /// The gender fem variant.
    GenderFem,
    /// The gender neut variant.
    GenderNeut,
    /// The definiteness def variant.
    DefinitenessDef,
    /// The definiteness ind variant.
    DefinitenessInd,
    /// The polarity neg variant.
    PolarityNeg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for morph tag set.
pub struct MorphTagSet {
    /// The features value.
    pub features: BTreeSet<MorphFeature>,
}

impl MorphTagSet {
    /// Creates a new value.
    pub fn new() -> Self {
        Self {
            features: BTreeSet::new(),
        }
    }

    /// Returns insert.
    pub fn insert(&mut self, feature: MorphFeature) {
        self.features.insert(feature);
    }
}

impl Default for MorphTagSet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for morph annotation.
pub struct MorphAnnotation {
    /// The token index value.
    pub token_index: usize,
    /// The lemma value.
    pub lemma: Option<String>,
    /// The tags value.
    pub tags: MorphTagSet,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing pos tag.
pub enum PosTag {
    /// The adj variant.
    Adj,
    /// The adp variant.
    Adp,
    /// The adv variant.
    Adv,
    /// The aux variant.
    Aux,
    /// The cconj variant.
    Cconj,
    /// The det variant.
    Det,
    /// The intj variant.
    Intj,
    /// The noun variant.
    Noun,
    /// The num variant.
    Num,
    /// The part variant.
    Part,
    /// The pron variant.
    Pron,
    /// The propn variant.
    Propn,
    /// The punct variant.
    Punct,
    /// The sconj variant.
    Sconj,
    /// The sym variant.
    Sym,
    /// The verb variant.
    Verb,
    /// The x variant.
    X,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for pos annotation.
pub struct PosAnnotation {
    /// The token index value.
    pub token_index: usize,
    /// The tag value.
    pub tag: PosTag,
    /// Confidence score for this value.
    pub confidence: f32,
    /// The reason value.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for pos tagging options.
pub struct PosTaggingOptions {
    /// Language tag for this value.
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// Data type for pos tagger.
pub struct PosTagger {
    /// The options value.
    pub options: PosTaggingOptions,
}

impl PosTagger {
    /// Returns tag tokens.
    pub fn tag_tokens(&self, tokens: &[Token], lemmas: &LemmatizationResult) -> Vec<PosAnnotation> {
        tokens
            .iter()
            .enumerate()
            .map(|(index, token)| {
                let lemma = lemmas.lemmas.get(index).map(|lemma| lemma.value.as_str());
                heuristic_pos(token, lemma, index, index == 0)
            })
            .collect()
    }
}

/// Returns annotate morphology.
pub fn annotate_morphology(
    tokens: &[Token],
    lemmas: &LemmatizationResult,
    pos: &[PosAnnotation],
) -> Vec<MorphAnnotation> {
    tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            let mut tags = MorphTagSet::new();
            let lemma = lemmas.lemmas.get(index).map(|lemma| lemma.value.clone());
            let pos_tag = pos
                .get(index)
                .map(|annotation| annotation.tag)
                .unwrap_or(PosTag::X);
            let normalized = token.normalized.as_str();
            match pos_tag {
                PosTag::Pron => {
                    if ["i", "me", "my", "we", "us", "our"].contains(&normalized) {
                        tags.insert(MorphFeature::Person1);
                    } else if ["you", "your"].contains(&normalized) {
                        tags.insert(MorphFeature::Person2);
                    } else {
                        tags.insert(MorphFeature::Person3);
                    }
                    if ["they", "them", "their", "we", "us", "our"].contains(&normalized) {
                        tags.insert(MorphFeature::NumberPlur);
                    } else {
                        tags.insert(MorphFeature::NumberSing);
                    }
                    if ["he", "him", "his"].contains(&normalized) {
                        tags.insert(MorphFeature::GenderMasc);
                    }
                    if ["she", "her", "hers"].contains(&normalized) {
                        tags.insert(MorphFeature::GenderFem);
                    }
                    if ["it", "its"].contains(&normalized) {
                        tags.insert(MorphFeature::GenderNeut);
                    }
                    tags.insert(MorphFeature::CaseNom);
                }
                PosTag::Det => {
                    if ["the", "this", "that", "these", "those"].contains(&normalized) {
                        tags.insert(MorphFeature::DefinitenessDef);
                    } else {
                        tags.insert(MorphFeature::DefinitenessInd);
                    }
                }
                PosTag::Noun | PosTag::Propn => {
                    if normalized.ends_with('s') && !normalized.ends_with("ss") {
                        tags.insert(MorphFeature::NumberPlur);
                    } else {
                        tags.insert(MorphFeature::NumberSing);
                    }
                }
                PosTag::Verb | PosTag::Aux => {
                    if normalized.ends_with("ed")
                        || ["was", "were", "did", "had"].contains(&normalized)
                    {
                        tags.insert(MorphFeature::TensePast);
                    } else if normalized == "will" || normalized == "shall" {
                        tags.insert(MorphFeature::TenseFut);
                    } else {
                        tags.insert(MorphFeature::TensePres);
                    }
                    if normalized.ends_with("ing") {
                        tags.insert(MorphFeature::AspectProg);
                    }
                    if ["have", "has", "had"].contains(&normalized) {
                        tags.insert(MorphFeature::AspectPerf);
                    }
                    if normalized == "to" {
                        tags.insert(MorphFeature::VerbFormInf);
                    } else {
                        tags.insert(MorphFeature::VerbFormFin);
                    }
                }
                _ => {}
            }
            if ["not", "never", "no"].contains(&normalized) {
                tags.insert(MorphFeature::PolarityNeg);
            }
            MorphAnnotation {
                token_index: index,
                lemma,
                tags,
                confidence: if lemmas.language.as_deref() == Some("en") {
                    0.75
                } else {
                    0.4
                },
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing chunk kind.
pub enum ChunkKind {
    /// The noun phrase variant.
    NounPhrase,
    /// The verb phrase variant.
    VerbPhrase,
    /// The prep phrase variant.
    PrepPhrase,
    /// The adjective phrase variant.
    AdjectivePhrase,
    /// The adverb phrase variant.
    AdverbPhrase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for phrase chunk.
pub struct PhraseChunk {
    /// The kind value.
    pub kind: ChunkKind,
    /// The sentence index value.
    pub sentence_index: usize,
    /// The token start value.
    pub token_start: usize,
    /// The token end value.
    pub token_end: usize,
    /// The head token index value.
    pub head_token_index: usize,
    /// Text content for this value.
    pub text: String,
    /// The span value.
    pub span: TextSpan,
}

/// Returns chunk phrases.
pub fn chunk_phrases(
    text: &str,
    sentences: &[Sentence],
    tokens: &[Token],
    pos: &[PosAnnotation],
) -> Vec<PhraseChunk> {
    let sentence_ranges = sentence_token_ranges(sentences, tokens);
    let mut chunks = Vec::new();
    for (sentence_index, token_range) in sentence_ranges.into_iter().enumerate() {
        let indices = (token_range.0..token_range.1).collect::<Vec<_>>();
        let mut cursor = 0;
        while cursor < indices.len() {
            let token_index = indices[cursor];
            let tag = pos
                .get(token_index)
                .map(|annotation| annotation.tag)
                .unwrap_or(PosTag::X);

            let maybe_chunk = if matches!(
                tag,
                PosTag::Det | PosTag::Adj | PosTag::Noun | PosTag::Propn | PosTag::Pron
            ) {
                let mut end = cursor;
                while end + 1 < indices.len() {
                    let next_tag = pos
                        .get(indices[end + 1])
                        .map(|annotation| annotation.tag)
                        .unwrap_or(PosTag::X);
                    if matches!(
                        next_tag,
                        PosTag::Adj | PosTag::Noun | PosTag::Propn | PosTag::Pron
                    ) {
                        end += 1;
                    } else {
                        break;
                    }
                }
                let start_index = indices[cursor];
                let end_index = indices[end];
                Some((ChunkKind::NounPhrase, start_index, end_index, end_index))
            } else if matches!(tag, PosTag::Aux | PosTag::Verb | PosTag::Adv) {
                let mut end = cursor;
                while end + 1 < indices.len() {
                    let next_tag = pos
                        .get(indices[end + 1])
                        .map(|annotation| annotation.tag)
                        .unwrap_or(PosTag::X);
                    if matches!(
                        next_tag,
                        PosTag::Aux | PosTag::Verb | PosTag::Adv | PosTag::Part
                    ) {
                        end += 1;
                    } else {
                        break;
                    }
                }
                let start_index = indices[cursor];
                let end_index = indices[end];
                let head = indices[cursor..=end]
                    .iter()
                    .copied()
                    .find(|index| matches!(pos[*index].tag, PosTag::Verb | PosTag::Aux))
                    .unwrap_or(start_index);
                Some((ChunkKind::VerbPhrase, start_index, end_index, head))
            } else if tag == PosTag::Adp {
                let start_index = token_index;
                let mut end = cursor;
                while end + 1 < indices.len() {
                    let next_tag = pos
                        .get(indices[end + 1])
                        .map(|annotation| annotation.tag)
                        .unwrap_or(PosTag::X);
                    if matches!(
                        next_tag,
                        PosTag::Det | PosTag::Adj | PosTag::Noun | PosTag::Propn | PosTag::Pron
                    ) {
                        end += 1;
                    } else {
                        break;
                    }
                }
                let end_index = indices[end];
                Some((ChunkKind::PrepPhrase, start_index, end_index, token_index))
            } else {
                None
            };

            if let Some((kind, start_index, end_index, head_index)) = maybe_chunk {
                let span = TextSpan {
                    byte_start: tokens[start_index].span.byte_start,
                    byte_end: tokens[end_index].span.byte_end,
                    char_start: tokens[start_index].span.char_start,
                    char_end: tokens[end_index].span.char_end,
                };
                chunks.push(PhraseChunk {
                    kind,
                    sentence_index,
                    token_start: start_index,
                    token_end: end_index + 1,
                    head_token_index: head_index,
                    text: text[span.byte_start..span.byte_end].to_string(),
                    span,
                });
                cursor = indices
                    .iter()
                    .position(|index| *index == end_index)
                    .map(|position| position + 1)
                    .unwrap_or(indices.len());
            } else {
                cursor += 1;
            }
        }
    }
    chunks
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing dependency relation.
pub enum DependencyRelation {
    /// The root variant.
    Root,
    /// The nsubj variant.
    Nsubj,
    /// The obj variant.
    Obj,
    /// The iobj variant.
    Iobj,
    /// The obl variant.
    Obl,
    /// The advmod variant.
    Advmod,
    /// The amod variant.
    Amod,
    /// The det variant.
    Det,
    /// The case variant.
    Case,
    /// The aux variant.
    Aux,
    /// The compound variant.
    Compound,
    /// The cc variant.
    Cc,
    /// The conj variant.
    Conj,
    /// The appos variant.
    Appos,
    /// The nmod variant.
    Nmod,
    /// The dep variant.
    Dep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for dependency node.
pub struct DependencyNode {
    /// The token index value.
    pub token_index: usize,
    /// The head token index value.
    pub head_token_index: Option<usize>,
    /// The relation value.
    pub relation: DependencyRelation,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for dependency edge.
pub struct DependencyEdge {
    /// The head token index value.
    pub head_token_index: usize,
    /// The dependent token index value.
    pub dependent_token_index: usize,
    /// The relation value.
    pub relation: DependencyRelation,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for dependency tree.
pub struct DependencyTree {
    /// The sentence index value.
    pub sentence_index: usize,
    /// The root token index value.
    pub root_token_index: Option<usize>,
    /// The nodes value.
    pub nodes: Vec<DependencyNode>,
    /// The edges value.
    pub edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for dependency parser.
pub struct DependencyParser;

impl DependencyParser {
    /// Parses parse document.
    pub fn parse_document(
        &self,
        sentences: &[Sentence],
        tokens: &[Token],
        pos: &[PosAnnotation],
    ) -> Vec<DependencyTree> {
        sentence_token_ranges(sentences, tokens)
            .into_iter()
            .enumerate()
            .map(|(sentence_index, (start, end))| {
                self.parse_sentence(sentence_index, tokens, pos, start, end)
            })
            .collect()
    }

    fn parse_sentence(
        &self,
        sentence_index: usize,
        tokens: &[Token],
        pos: &[PosAnnotation],
        start: usize,
        end: usize,
    ) -> DependencyTree {
        let indices = (start..end).collect::<Vec<_>>();
        let root_token_index = indices
            .iter()
            .copied()
            .find(|index| matches!(pos[*index].tag, PosTag::Verb | PosTag::Aux))
            .or_else(|| {
                indices
                    .iter()
                    .copied()
                    .find(|index| matches!(pos[*index].tag, PosTag::Noun | PosTag::Propn))
            })
            .or_else(|| indices.first().copied());

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for token_index in indices.iter().copied() {
            let tag = pos
                .get(token_index)
                .map(|annotation| annotation.tag)
                .unwrap_or(PosTag::X);
            let (head, relation, confidence) = if Some(token_index) == root_token_index {
                (None, DependencyRelation::Root, 1.0)
            } else if matches!(tag, PosTag::Det) {
                let head = nearest_following(tokens, pos, token_index, end, |tag| {
                    matches!(tag, PosTag::Noun | PosTag::Propn | PosTag::Pron)
                })
                .or(root_token_index);
                (head, DependencyRelation::Det, 0.8)
            } else if matches!(tag, PosTag::Adj) {
                let head = nearest_following(tokens, pos, token_index, end, |tag| {
                    matches!(tag, PosTag::Noun | PosTag::Propn)
                })
                .or(root_token_index);
                (head, DependencyRelation::Amod, 0.7)
            } else if matches!(tag, PosTag::Adp) {
                let head = nearest_following(tokens, pos, token_index, end, |tag| {
                    matches!(tag, PosTag::Noun | PosTag::Propn | PosTag::Pron)
                })
                .or(root_token_index);
                (head, DependencyRelation::Case, 0.7)
            } else if matches!(tag, PosTag::Aux) {
                (root_token_index, DependencyRelation::Aux, 0.85)
            } else if matches!(tag, PosTag::Adv) {
                (root_token_index, DependencyRelation::Advmod, 0.7)
            } else if matches!(tag, PosTag::Cconj | PosTag::Sconj) {
                (root_token_index, DependencyRelation::Cc, 0.6)
            } else if matches!(tag, PosTag::Noun | PosTag::Propn | PosTag::Pron) {
                if let Some(root) = root_token_index {
                    if token_index < root {
                        (Some(root), DependencyRelation::Nsubj, 0.75)
                    } else {
                        (Some(root), DependencyRelation::Obj, 0.7)
                    }
                } else {
                    (None, DependencyRelation::Dep, 0.4)
                }
            } else {
                (root_token_index, DependencyRelation::Dep, 0.4)
            };
            nodes.push(DependencyNode {
                token_index,
                head_token_index: head,
                relation,
            });
            if let Some(head) = head {
                edges.push(DependencyEdge {
                    head_token_index: head,
                    dependent_token_index: token_index,
                    relation,
                    confidence,
                });
            }
        }

        DependencyTree {
            sentence_index,
            root_token_index,
            nodes,
            edges,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Variants describing entity type.
pub enum EntityType {
    /// The person variant.
    Person,
    /// The organization variant.
    Organization,
    /// The location variant.
    Location,
    /// The product variant.
    Product,
    /// The date variant.
    Date,
    /// The amount variant.
    Amount,
    /// The law variant.
    Law,
    /// The work variant.
    Work,
    /// The event variant.
    Event,
    /// The misc variant.
    Misc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for entity mention span.
pub struct EntityMentionSpan {
    /// Text content for this value.
    pub text: String,
    /// The span value.
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for named entity.
pub struct NamedEntity {
    /// Identifier for this value.
    pub id: String,
    /// The entity type value.
    pub entity_type: EntityType,
    /// The mention value.
    pub mention: EntityMentionSpan,
    /// The normalized value.
    pub normalized: String,
    /// The sentence index value.
    pub sentence_index: usize,
    /// The token start value.
    pub token_start: usize,
    /// The token end value.
    pub token_end: usize,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for canonical entity.
pub struct CanonicalEntity {
    /// Identifier for this value.
    pub id: String,
    /// The entity type value.
    pub entity_type: EntityType,
    /// The canonical name value.
    pub canonical_name: String,
    /// The aliases value.
    pub aliases: Vec<String>,
    /// The mentions value.
    pub mentions: Vec<NamedEntity>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for entity linking options.
pub struct EntityLinkingOptions {
    /// The lowercase keys value.
    pub lowercase_keys: bool,
}

impl Default for EntityLinkingOptions {
    fn default() -> Self {
        Self {
            lowercase_keys: true,
        }
    }
}

/// Returns extract named entities.
pub fn extract_named_entities(
    text: &str,
    sentences: &[Sentence],
    tokens: &[Token],
    pos: &[PosAnnotation],
) -> Vec<NamedEntity> {
    let sentence_ranges = sentence_token_ranges(sentences, tokens);
    let mut entities = Vec::new();
    let mut next_id = 0_usize;

    for (sentence_index, (start, end)) in sentence_ranges.into_iter().enumerate() {
        let mut cursor = start;
        while cursor < end {
            let tag = pos
                .get(cursor)
                .map(|annotation| annotation.tag)
                .unwrap_or(PosTag::X);
            let token = &tokens[cursor];
            if token.text.starts_with('$') && token.text[1..].chars().any(|ch| ch.is_ascii_digit())
            {
                entities.push(build_entity(
                    next_id,
                    EntityType::Amount,
                    text,
                    sentence_index,
                    cursor..(cursor + 1),
                    tokens,
                    0.9,
                ));
                next_id += 1;
                cursor += 1;
                continue;
            }
            if is_date_token(token) {
                entities.push(build_entity(
                    next_id,
                    EntityType::Date,
                    text,
                    sentence_index,
                    cursor..(cursor + 1),
                    tokens,
                    0.85,
                ));
                next_id += 1;
                cursor += 1;
                continue;
            }
            if matches!(tag, PosTag::Propn) {
                let start_index = cursor;
                let mut end_index = cursor + 1;
                while end_index < end {
                    let next_tag = pos
                        .get(end_index)
                        .map(|annotation| annotation.tag)
                        .unwrap_or(PosTag::X);
                    if matches!(next_tag, PosTag::Propn | PosTag::Noun) {
                        end_index += 1;
                    } else {
                        break;
                    }
                }
                let entity_type = classify_capitalized_entity(&tokens[start_index..end_index]);
                entities.push(build_entity(
                    next_id,
                    entity_type,
                    text,
                    sentence_index,
                    start_index..end_index,
                    tokens,
                    0.7,
                ));
                next_id += 1;
                cursor = end_index;
                continue;
            }
            cursor += 1;
        }
    }

    entities
}

/// Returns canonicalize entities.
pub fn canonicalize_entities(
    entities: &[NamedEntity],
    options: &EntityLinkingOptions,
) -> Vec<CanonicalEntity> {
    let mut grouped = BTreeMap::<(EntityType, String), Vec<NamedEntity>>::new();
    for entity in entities {
        let key = if options.lowercase_keys {
            entity.normalized.to_lowercase()
        } else {
            entity.normalized.clone()
        };
        grouped
            .entry((entity.entity_type, key))
            .or_default()
            .push(entity.clone());
    }
    grouped
        .into_iter()
        .enumerate()
        .map(|(index, ((entity_type, key), mentions))| CanonicalEntity {
            id: format!("entity-{index}"),
            entity_type,
            canonical_name: mentions
                .first()
                .map(|mention| mention.mention.text.clone())
                .unwrap_or(key.clone()),
            aliases: mentions
                .iter()
                .map(|mention| mention.mention.text.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            mentions,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for coref mention.
pub struct CorefMention {
    /// Text content for this value.
    pub text: String,
    /// The sentence index value.
    pub sentence_index: usize,
    /// The token start value.
    pub token_start: usize,
    /// The token end value.
    pub token_end: usize,
    /// The entity type value.
    pub entity_type: Option<EntityType>,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for coref cluster.
pub struct CorefCluster {
    /// Identifier for this value.
    pub id: String,
    /// The canonical text value.
    pub canonical_text: String,
    /// The entity type value.
    pub entity_type: Option<EntityType>,
    /// The mentions value.
    pub mentions: Vec<CorefMention>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for coref resolver.
pub struct CorefResolver {
    /// The speaker aware value.
    pub speaker_aware: bool,
}

impl CorefResolver {
    /// Returns resolve.
    pub fn resolve(
        &self,
        tokens: &[Token],
        canonical_entities: &[CanonicalEntity],
    ) -> Vec<CorefCluster> {
        let mut clusters = canonical_entities
            .iter()
            .map(|entity| CorefCluster {
                id: entity.id.clone(),
                canonical_text: entity.canonical_name.clone(),
                entity_type: Some(entity.entity_type),
                mentions: entity
                    .mentions
                    .iter()
                    .map(|mention| CorefMention {
                        text: mention.mention.text.clone(),
                        sentence_index: mention.sentence_index,
                        token_start: mention.token_start,
                        token_end: mention.token_end,
                        entity_type: Some(mention.entity_type),
                        confidence: mention.confidence,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();

        for (index, token) in tokens.iter().enumerate() {
            if token.kind != TokenKind::Word {
                continue;
            }
            let normalized = token.normalized.as_str();
            let pronoun_type = match normalized {
                "he" | "him" | "his" | "she" | "her" | "hers" | "they" | "them" | "their" => {
                    Some(EntityType::Person)
                }
                "it" | "its" => Some(EntityType::Misc),
                _ => None,
            };
            let Some(entity_type) = pronoun_type else {
                continue;
            };
            if let Some(cluster) = clusters.iter_mut().rev().find(|cluster| {
                cluster.entity_type == Some(entity_type)
                    || cluster.entity_type == Some(EntityType::Organization)
            }) {
                cluster.mentions.push(CorefMention {
                    text: token.text.clone(),
                    sentence_index: 0,
                    token_start: index,
                    token_end: index + 1,
                    entity_type: Some(entity_type),
                    confidence: 0.55,
                });
            }
        }

        clusters
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing relation type.
pub enum RelationType {
    /// The action variant.
    Action,
    /// The attribution variant.
    Attribution,
    /// The temporal variant.
    Temporal,
    /// The causal variant.
    Causal,
    /// The location variant.
    Location,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for event argument.
pub struct EventArgument {
    /// The role value.
    pub role: String,
    /// Text content for this value.
    pub text: String,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for relation triple.
pub struct RelationTriple {
    /// The subject value.
    pub subject: String,
    /// The relation value.
    pub relation: String,
    /// The object value.
    pub object: String,
    /// The relation type value.
    pub relation_type: RelationType,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for extracted event.
pub struct ExtractedEvent {
    /// The sentence index value.
    pub sentence_index: usize,
    /// The predicate value.
    pub predicate: String,
    /// The lemma value.
    pub lemma: String,
    /// The relation type value.
    pub relation_type: RelationType,
    /// The arguments value.
    pub arguments: Vec<EventArgument>,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for event extractor.
pub struct EventExtractor;

impl EventExtractor {
    /// Returns extract.
    pub fn extract(
        &self,
        trees: &[DependencyTree],
        tokens: &[Token],
        lemmas: &LemmatizationResult,
    ) -> (Vec<ExtractedEvent>, Vec<RelationTriple>) {
        let mut events = Vec::new();
        let mut relations = Vec::new();
        for tree in trees {
            let Some(root_index) = tree.root_token_index else {
                continue;
            };
            let predicate = tokens[root_index].text.clone();
            let lemma = lemmas
                .lemmas
                .get(root_index)
                .map(|lemma| lemma.value.clone())
                .unwrap_or_else(|| tokens[root_index].normalized.clone());
            let subject = tree
                .edges
                .iter()
                .find(|edge| edge.relation == DependencyRelation::Nsubj)
                .map(|edge| tokens[edge.dependent_token_index].text.clone());
            let object = tree
                .edges
                .iter()
                .find(|edge| {
                    matches!(
                        edge.relation,
                        DependencyRelation::Obj | DependencyRelation::Obl
                    )
                })
                .map(|edge| tokens[edge.dependent_token_index].text.clone());
            let relation_type =
                if ["say", "announce", "report", "present"].contains(&lemma.as_str()) {
                    RelationType::Attribution
                } else if ["because", "cause"].contains(&lemma.as_str()) {
                    RelationType::Causal
                } else if ["visit", "go", "travel"].contains(&lemma.as_str()) {
                    RelationType::Location
                } else {
                    RelationType::Action
                };
            let mut arguments = Vec::new();
            if let Some(subject) = subject.clone() {
                arguments.push(EventArgument {
                    role: "subject".to_string(),
                    text: subject,
                    confidence: 0.75,
                });
            }
            if let Some(object) = object.clone() {
                arguments.push(EventArgument {
                    role: "object".to_string(),
                    text: object,
                    confidence: 0.7,
                });
            }
            if !arguments.is_empty() {
                events.push(ExtractedEvent {
                    sentence_index: tree.sentence_index,
                    predicate: predicate.clone(),
                    lemma: lemma.clone(),
                    relation_type,
                    arguments: arguments.clone(),
                    confidence: 0.7,
                });
            }
            if let (Some(subject), Some(object)) = (subject, object) {
                relations.push(RelationTriple {
                    subject,
                    relation: lemma,
                    object,
                    relation_type,
                    confidence: 0.7,
                });
            }
        }
        (events, relations)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing discourse relation.
pub enum DiscourseRelation {
    /// The sequence variant.
    Sequence,
    /// The elaboration variant.
    Elaboration,
    /// The contrast variant.
    Contrast,
    /// The cause variant.
    Cause,
    /// The question answer variant.
    QuestionAnswer,
    /// The transition variant.
    Transition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing discourse segment kind.
pub enum DiscourseSegmentKind {
    /// The intro variant.
    Intro,
    /// The body variant.
    Body,
    /// The conclusion variant.
    Conclusion,
    /// The question variant.
    Question,
    /// The answer variant.
    Answer,
    /// The claim variant.
    Claim,
    /// The evidence variant.
    Evidence,
    /// The topic shift variant.
    TopicShift,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for discourse segment.
pub struct DiscourseSegment {
    /// The index value.
    pub index: usize,
    /// The kind value.
    pub kind: DiscourseSegmentKind,
    /// The sentence start value.
    pub sentence_start: usize,
    /// The sentence end value.
    pub sentence_end: usize,
    /// Text content for this value.
    pub text: String,
    /// The cues value.
    pub cues: Vec<String>,
    /// The relation to previous value.
    pub relation_to_previous: Option<DiscourseRelation>,
    /// Confidence score for this value.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for document outline.
pub struct DocumentOutline {
    /// The segments value.
    pub segments: Vec<DiscourseSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for section classifier.
pub struct SectionClassifier;

impl SectionClassifier {
    /// Returns classify.
    pub fn classify(&self, sentences: &[Sentence]) -> DocumentOutline {
        let mut segments = Vec::new();
        for (index, sentence) in sentences.iter().enumerate() {
            let lower = sentence.text.to_lowercase();
            let mut cues = Vec::new();
            let mut kind = if index == 0 {
                DiscourseSegmentKind::Intro
            } else if index + 1 == sentences.len() {
                DiscourseSegmentKind::Conclusion
            } else {
                DiscourseSegmentKind::Body
            };
            let mut relation_to_previous = if index > 0 {
                Some(DiscourseRelation::Sequence)
            } else {
                None
            };
            if lower.trim_end().ends_with('?') {
                kind = DiscourseSegmentKind::Question;
                relation_to_previous = Some(DiscourseRelation::QuestionAnswer);
                cues.push("question_mark".to_string());
            } else if lower.starts_with("yes") || lower.starts_with("no") {
                kind = DiscourseSegmentKind::Answer;
                relation_to_previous = Some(DiscourseRelation::QuestionAnswer);
                cues.push("answer_cue".to_string());
            } else if lower.contains("because") || lower.contains("since") {
                kind = DiscourseSegmentKind::Evidence;
                relation_to_previous = Some(DiscourseRelation::Cause);
                cues.push("causal_marker".to_string());
            } else if lower.contains("however") || lower.contains("but ") {
                kind = DiscourseSegmentKind::Claim;
                relation_to_previous = Some(DiscourseRelation::Contrast);
                cues.push("contrast_marker".to_string());
            } else if lower.starts_with("first")
                || lower.starts_with("next")
                || lower.starts_with("finally")
            {
                kind = DiscourseSegmentKind::TopicShift;
                relation_to_previous = Some(DiscourseRelation::Transition);
                cues.push("transition_marker".to_string());
            }
            segments.push(DiscourseSegment {
                index,
                kind,
                sentence_start: index,
                sentence_end: index + 1,
                text: sentence.text.clone(),
                cues,
                relation_to_previous,
                confidence: 0.7,
            });
        }
        DocumentOutline { segments }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for topic descriptor.
pub struct TopicDescriptor {
    /// Label assigned to this value.
    pub label: String,
    /// The terms value.
    pub terms: Vec<String>,
    /// Score assigned to this value.
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for topic cluster.
pub struct TopicCluster {
    /// Identifier for this value.
    pub id: String,
    /// The descriptor value.
    pub descriptor: TopicDescriptor,
    /// The sentence indices value.
    pub sentence_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for topic model.
pub struct TopicModel {
    /// The descriptors value.
    pub descriptors: Vec<TopicDescriptor>,
    /// The clusters value.
    pub clusters: Vec<TopicCluster>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing register estimate.
pub enum RegisterEstimate {
    /// The formal variant.
    Formal,
    /// The neutral variant.
    Neutral,
    /// The informal variant.
    Informal,
    /// The technical variant.
    Technical,
    /// The conversational variant.
    Conversational,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for complexity metrics.
pub struct ComplexityMetrics {
    /// The average sentence tokens value.
    pub average_sentence_tokens: f32,
    /// The clause density value.
    pub clause_density: f32,
    /// The type token ratio value.
    pub type_token_ratio: f32,
    /// The lemma type token ratio value.
    pub lemma_type_token_ratio: f32,
    /// The disfluency rate value.
    pub disfluency_rate: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for style profile.
pub struct StyleProfile {
    /// The register value.
    pub register: RegisterEstimate,
    /// The complexity value.
    pub complexity: ComplexityMetrics,
    /// The question count value.
    pub question_count: usize,
    /// The exclamation count value.
    pub exclamation_count: usize,
    /// The passive voice estimate value.
    pub passive_voice_estimate: f32,
    /// The formality score value.
    pub formality_score: f32,
    /// The repetitiveness value.
    pub repetitiveness: f32,
    /// The disfluency markers value.
    pub disfluency_markers: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing analysis profile.
pub enum AnalysisProfile {
    /// The fast variant.
    Fast,
    /// The balanced variant.
    Balanced,
    #[default]
    /// The rich variant.
    Rich,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for linguistic analysis options.
pub struct LinguisticAnalysisOptions {
    /// The processing value.
    pub processing: TextProcessingOptions,
    /// The language detection value.
    pub language_detection: LanguageDetectionOptions,
    /// The tokenizer policy value.
    pub tokenizer_policy: TokenizerPolicy,
    /// The lemma value.
    pub lemma: LemmaOptions,
    /// The pos value.
    pub pos: PosTaggingOptions,
    /// The entity linking value.
    pub entity_linking: EntityLinkingOptions,
    /// The enable alignment value.
    pub enable_alignment: bool,
    /// The enable coreference value.
    pub enable_coreference: bool,
    /// The enable events value.
    pub enable_events: bool,
    /// The enable discourse value.
    pub enable_discourse: bool,
    /// The enable topics value.
    pub enable_topics: bool,
    /// The enable style value.
    pub enable_style: bool,
}

impl Default for LinguisticAnalysisOptions {
    fn default() -> Self {
        Self {
            processing: TextProcessingOptions {
                include_punctuation: true,
                ..TextProcessingOptions::default()
            },
            language_detection: LanguageDetectionOptions::default(),
            tokenizer_policy: TokenizerPolicy::default(),
            lemma: LemmaOptions::default(),
            pos: PosTaggingOptions::default(),
            entity_linking: EntityLinkingOptions::default(),
            enable_alignment: false,
            enable_coreference: true,
            enable_events: true,
            enable_discourse: true,
            enable_topics: true,
            enable_style: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for text nlp config.
pub struct TextNlpConfig {
    /// The profile value.
    pub profile: AnalysisProfile,
    /// The options value.
    pub options: LinguisticAnalysisOptions,
    /// The prefer model backends value.
    pub prefer_model_backends: bool,
    /// The model family value.
    pub model_family: Option<String>,
}

impl Default for TextNlpConfig {
    fn default() -> Self {
        Self::rich()
    }
}

impl TextNlpConfig {
    /// Returns fast.
    pub fn fast() -> Self {
        Self {
            profile: AnalysisProfile::Fast,
            options: options_for_profile(AnalysisProfile::Fast),
            prefer_model_backends: false,
            model_family: None,
        }
    }

    /// Returns balanced.
    pub fn balanced() -> Self {
        Self {
            profile: AnalysisProfile::Balanced,
            options: options_for_profile(AnalysisProfile::Balanced),
            prefer_model_backends: true,
            model_family: None,
        }
    }

    /// Returns rich.
    pub fn rich() -> Self {
        Self {
            profile: AnalysisProfile::Rich,
            options: options_for_profile(AnalysisProfile::Rich),
            prefer_model_backends: true,
            model_family: Some("default-rich".to_string()),
        }
    }

    /// Builds this value from options.
    pub fn from_options(options: LinguisticAnalysisOptions) -> Self {
        Self {
            profile: AnalysisProfile::Balanced,
            options,
            prefer_model_backends: true,
            model_family: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for text nlp pipeline.
pub struct TextNlpPipeline {
    config: TextNlpConfig,
}

impl Default for TextNlpPipeline {
    fn default() -> Self {
        Self::new(TextNlpConfig::default())
    }
}

impl TextNlpPipeline {
    /// Creates a new value.
    pub fn new(config: TextNlpConfig) -> Self {
        Self { config }
    }

    /// Returns config.
    pub fn config(&self) -> &TextNlpConfig {
        &self.config
    }

    /// Returns analyze text.
    pub fn analyze_text(&self, text: &str) -> Result<LinguisticAnalysis> {
        analyze_text_with_config(text, &self.config)
    }

    /// Returns analyze document.
    pub fn analyze_document(&self, document: &TextDocument<'_>) -> Result<LinguisticAnalysis> {
        self.analyze_text(document.text)
    }

    /// Returns analyze segment.
    pub fn analyze_segment(&self, segment: &TextSegment<'_>) -> Result<LinguisticAnalysis> {
        self.analyze_text(segment.text)
    }

    /// Returns analyze subtitle segments.
    pub fn analyze_subtitle_segments(
        &self,
        segments: &[TranscriptSegment],
    ) -> Result<SubtitleLinguisticAnalysis> {
        let cues = segments
            .iter()
            .cloned()
            .map(|cue| {
                let analysis = self.analyze_text(&cue.text)?;
                Ok(SubtitleCueLinguisticAnalysis { cue, analysis })
            })
            .collect::<Result<Vec<_>>>()?;
        let aggregate = self.analyze_text(&join_subtitle_text(segments, None))?;
        Ok(SubtitleLinguisticAnalysis { cues, aggregate })
    }

    /// Returns analyze transcription.
    pub fn analyze_transcription(
        &self,
        result: &TranscriptionResult,
    ) -> Result<SubtitleLinguisticAnalysis> {
        let cues = result
            .segments
            .iter()
            .cloned()
            .map(|cue| {
                let analysis = self.analyze_text(&cue.text)?;
                Ok(SubtitleCueLinguisticAnalysis { cue, analysis })
            })
            .collect::<Result<Vec<_>>>()?;
        let aggregate = self.analyze_text(&join_subtitle_text(
            &result.segments,
            result.text.as_deref(),
        ))?;
        Ok(SubtitleLinguisticAnalysis { cues, aggregate })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for linguistic analysis.
pub struct LinguisticAnalysis {
    /// The profile value.
    pub profile: AnalysisProfile,
    /// The provenance value.
    pub provenance: AnnotationProvenance,
    /// Confidence score for this value.
    pub confidence: AnnotationConfidence,
    /// The graph value.
    pub graph: TextAnnotationGraph,
    /// Language tag for this value.
    pub language: LanguageProfile,
    /// The tokenizer value.
    pub tokenizer: TokenizerSelection,
    /// The sentences value.
    pub sentences: Vec<Sentence>,
    /// The tokens value.
    pub tokens: Vec<Token>,
    /// The alignments value.
    pub alignments: Option<TokenAlignmentMap>,
    /// The lemmas value.
    pub lemmas: Vec<Lemma>,
    /// The morphology value.
    pub morphology: Vec<MorphAnnotation>,
    /// The pos value.
    pub pos: Vec<PosAnnotation>,
    /// The chunks value.
    pub chunks: Vec<PhraseChunk>,
    /// The dependencies value.
    pub dependencies: Vec<DependencyTree>,
    /// The entities value.
    pub entities: Vec<NamedEntity>,
    /// The canonical entities value.
    pub canonical_entities: Vec<CanonicalEntity>,
    /// The coreference value.
    pub coreference: Vec<CorefCluster>,
    /// The events value.
    pub events: Vec<ExtractedEvent>,
    /// The relations value.
    pub relations: Vec<RelationTriple>,
    /// The discourse value.
    pub discourse: Vec<DiscourseSegment>,
    /// The outline value.
    pub outline: DocumentOutline,
    /// The topics value.
    pub topics: TopicModel,
    /// The style value.
    pub style: StyleProfile,
}

impl LinguisticAnalysis {
    /// Returns token ref.
    pub fn token_ref(&self, token_index: usize) -> Option<&text_analysis_core::CanonicalToken> {
        self.graph.tokens.get(token_index)
    }

    /// Returns sentence ref.
    pub fn sentence_ref(
        &self,
        sentence_index: usize,
    ) -> Option<&text_analysis_core::AnnotatedSentence> {
        self.graph.sentences.get(sentence_index)
    }
}

fn options_for_profile(profile: AnalysisProfile) -> LinguisticAnalysisOptions {
    let mut options = LinguisticAnalysisOptions::default();
    match profile {
        AnalysisProfile::Fast => {
            options.tokenizer_policy.mode = TokenizationMode::Word;
            options.enable_alignment = false;
            options.enable_coreference = false;
            options.enable_events = false;
            options.enable_discourse = false;
            options.enable_topics = false;
            options.enable_style = false;
        }
        AnalysisProfile::Balanced => {}
        AnalysisProfile::Rich => {
            options.enable_alignment = true;
            options.enable_coreference = true;
            options.enable_events = true;
            options.enable_discourse = true;
            options.enable_topics = true;
            options.enable_style = true;
        }
    }
    options
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for subtitle cue linguistic analysis.
pub struct SubtitleCueLinguisticAnalysis {
    /// The cue value.
    pub cue: TranscriptSegment,
    /// The analysis value.
    pub analysis: LinguisticAnalysis,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for subtitle linguistic analysis.
pub struct SubtitleLinguisticAnalysis {
    /// The cues value.
    pub cues: Vec<SubtitleCueLinguisticAnalysis>,
    /// The aggregate value.
    pub aggregate: LinguisticAnalysis,
}

/// Returns analyze document.
pub fn analyze_document(
    document: &TextDocument<'_>,
    options: &LinguisticAnalysisOptions,
) -> Result<LinguisticAnalysis> {
    TextNlpPipeline::new(TextNlpConfig::from_options(options.clone())).analyze_document(document)
}

/// Returns analyze segment.
pub fn analyze_segment(
    segment: &TextSegment<'_>,
    options: &LinguisticAnalysisOptions,
) -> Result<LinguisticAnalysis> {
    TextNlpPipeline::new(TextNlpConfig::from_options(options.clone())).analyze_segment(segment)
}

/// Returns analyze subtitle segments.
pub fn analyze_subtitle_segments(
    segments: &[TranscriptSegment],
    options: &LinguisticAnalysisOptions,
) -> Result<SubtitleLinguisticAnalysis> {
    TextNlpPipeline::new(TextNlpConfig::from_options(options.clone()))
        .analyze_subtitle_segments(segments)
}

/// Returns analyze transcription.
pub fn analyze_transcription(
    result: &TranscriptionResult,
    options: &LinguisticAnalysisOptions,
) -> Result<SubtitleLinguisticAnalysis> {
    TextNlpPipeline::new(TextNlpConfig::from_options(options.clone())).analyze_transcription(result)
}

/// Returns analyze text.
pub fn analyze_text(text: &str, options: &LinguisticAnalysisOptions) -> Result<LinguisticAnalysis> {
    analyze_text_with_config(text, &TextNlpConfig::from_options(options.clone()))
}

fn analyze_text_with_config(text: &str, config: &TextNlpConfig) -> Result<LinguisticAnalysis> {
    let options = &config.options;
    let language_detector = LanguageDetector {
        options: options.language_detection.clone(),
        lexicons: LexiconStore::default(),
    };
    let language = language_detector.detect_text(text);
    let registry = TokenizerRegistry {
        policy: options.tokenizer_policy.clone(),
    };
    let tokenizer = registry.select(
        language
            .primary
            .as_ref()
            .map(|prediction| prediction.language.as_str()),
        Some("linguistic-analysis"),
        config.model_family.as_deref(),
    );

    let sentences = split_sentence_spans(text, &options.processing);
    let tokens = tokenize(text, &options.processing);
    let alignments = if options.enable_alignment {
        registry.align(text, &tokens, &tokenizer)?
    } else {
        None
    };
    let lemmas = lemmatize_tokens(
        &tokens,
        language
            .primary
            .as_ref()
            .map(|prediction| prediction.language.as_str()),
        &options.lemma,
    );
    let pos_tagger = PosTagger {
        options: options.pos.clone(),
    };
    let pos = pos_tagger.tag_tokens(&tokens, &lemmas);
    let morphology = annotate_morphology(&tokens, &lemmas, &pos);
    let chunks = chunk_phrases(text, &sentences, &tokens, &pos);
    let dependency_parser = DependencyParser;
    let dependencies = dependency_parser.parse_document(&sentences, &tokens, &pos);
    let entities = extract_named_entities(text, &sentences, &tokens, &pos);
    let canonical_entities = canonicalize_entities(&entities, &options.entity_linking);
    let coreference = if options.enable_coreference {
        CorefResolver::default().resolve(&tokens, &canonical_entities)
    } else {
        Vec::new()
    };
    let (events, relations) = if options.enable_events {
        EventExtractor.extract(&dependencies, &tokens, &lemmas)
    } else {
        (Vec::new(), Vec::new())
    };
    let outline = if options.enable_discourse {
        SectionClassifier.classify(&sentences)
    } else {
        DocumentOutline {
            segments: Vec::new(),
        }
    };
    let discourse = outline.segments.clone();
    let topics = if options.enable_topics {
        build_topic_model(
            &sentences,
            &tokens,
            &lemmas,
            &chunks,
            language.primary.as_ref(),
        )
    } else {
        TopicModel {
            descriptors: Vec::new(),
            clusters: Vec::new(),
        }
    };
    let style = if options.enable_style {
        build_style_profile(text, &sentences, &tokens, &lemmas.lemmas, &dependencies)
    } else {
        StyleProfile::default()
    };
    let graph =
        build_annotation_graph_from_parts(text, &tokens, &sentences, &split_paragraphs(text));
    let confidence = summarize_analysis_confidence(
        &language,
        &lemmas.lemmas,
        &pos,
        &entities,
        &events,
        alignments.as_ref(),
    );
    let provenance = analysis_provenance(
        &tokenizer,
        alignments.as_ref(),
        config.prefer_model_backends,
    );

    Ok(LinguisticAnalysis {
        profile: config.profile,
        provenance,
        confidence,
        graph,
        language,
        tokenizer,
        sentences,
        tokens,
        alignments,
        lemmas: lemmas.lemmas,
        morphology,
        pos,
        chunks,
        dependencies,
        entities,
        canonical_entities,
        coreference,
        events,
        relations,
        discourse,
        outline,
        topics,
        style,
    })
}

fn join_subtitle_text(segments: &[TranscriptSegment], aggregate_text: Option<&str>) -> String {
    if let Some(text) = aggregate_text.filter(|text| !text.trim().is_empty()) {
        return text.to_string();
    }

    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_analysis_confidence(
    language: &LanguageProfile,
    lemmas: &[Lemma],
    pos: &[PosAnnotation],
    entities: &[NamedEntity],
    events: &[ExtractedEvent],
    alignments: Option<&TokenAlignmentMap>,
) -> AnnotationConfidence {
    let mut values = Vec::new();
    if let Some(primary) = &language.primary {
        values.push(primary.confidence);
    }
    values.extend(lemmas.iter().map(|lemma| lemma.confidence));
    values.extend(pos.iter().map(|annotation| annotation.confidence));
    values.extend(entities.iter().map(|entity| entity.confidence));
    values.extend(events.iter().map(|event| event.confidence));
    if alignments.is_some() {
        values.push(0.85);
    }
    if values.is_empty() {
        AnnotationConfidence::new(0.0)
    } else {
        AnnotationConfidence::new(values.iter().sum::<f32>() / values.len() as f32)
    }
}

fn analysis_provenance(
    tokenizer: &TokenizerSelection,
    alignments: Option<&TokenAlignmentMap>,
    prefer_model_backends: bool,
) -> AnnotationProvenance {
    if alignments.is_some() || (prefer_model_backends && tokenizer.source.is_some()) {
        AnnotationProvenance::Tokenizer
    } else {
        AnnotationProvenance::Heuristic
    }
}

impl Default for StyleProfile {
    fn default() -> Self {
        Self {
            register: RegisterEstimate::Neutral,
            complexity: ComplexityMetrics {
                average_sentence_tokens: 0.0,
                clause_density: 0.0,
                type_token_ratio: 0.0,
                lemma_type_token_ratio: 0.0,
                disfluency_rate: 0.0,
            },
            question_count: 0,
            exclamation_count: 0,
            passive_voice_estimate: 0.0,
            formality_score: 0.0,
            repetitiveness: 0.0,
            disfluency_markers: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for linguistic analyzer.
pub struct LinguisticAnalyzer {
    options: LinguisticAnalysisOptions,
    segment_buffer: Vec<OwnedTextSegment>,
}

impl LinguisticAnalyzer {
    /// Creates a new value.
    pub fn new(options: LinguisticAnalysisOptions) -> Self {
        Self {
            options,
            segment_buffer: Vec::new(),
        }
    }

    fn segment_events(
        &self,
        segment: &TextSegment<'_>,
        analysis: &LinguisticAnalysis,
    ) -> Vec<AnalysisEvent> {
        let mut events = Vec::new();
        if let Some(language) = &analysis.language.primary {
            let mut event =
                AnalysisEvent::new(self.name(), format!("text:language:{}", language.language))
                    .score(language.confidence);
            if let Some(timestamp) = segment.timestamp {
                event = event.at_timestamp(timestamp);
            }
            events.push(event);
        }
        events.extend(analysis.entities.iter().map(|entity| {
            let mut event = AnalysisEvent::new(
                self.name(),
                format!(
                    "text:entity:{:?}:{}",
                    entity.entity_type,
                    entity.normalized.to_lowercase()
                ),
            )
            .score(entity.confidence);
            if let Some(timestamp) = segment.timestamp {
                event = event.at_timestamp(timestamp);
            }
            event
        }));
        events.extend(analysis.events.iter().map(|event_analysis| {
            let mut event = AnalysisEvent::new(
                self.name(),
                format!("text:event:{}", event_analysis.lemma.to_lowercase()),
            )
            .score(event_analysis.confidence);
            if let Some(timestamp) = segment.timestamp {
                event = event.at_timestamp(timestamp);
            }
            event
        }));
        events
    }

    fn document_events(&self) -> Result<Vec<AnalysisEvent>> {
        if self.segment_buffer.is_empty() {
            return Ok(Vec::new());
        }
        let joined = self
            .segment_buffer
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let analysis = analyze_text(&joined, &self.options)?;
        let mut events = Vec::new();
        for topic in &analysis.topics.descriptors {
            events.push(
                AnalysisEvent::new(self.name(), format!("text:topic:{}", topic.label))
                    .score(topic.score),
            );
        }
        for segment in &analysis.discourse {
            events.push(
                AnalysisEvent::new(
                    self.name(),
                    format!("text:discourse:{:?}", segment.kind).to_lowercase(),
                )
                .score(segment.confidence),
            );
        }
        Ok(events)
    }
}

impl TextAnalyzer for LinguisticAnalyzer {
    fn name(&self) -> &str {
        "linguistics"
    }

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
        let analysis = analyze_segment(segment, &self.options)?;
        self.segment_buffer.push(
            OwnedTextSegment::new(segment.segment_index, segment.text)
                .finality(segment.is_final)
                .language(segment.language.unwrap_or("und")),
        );
        Ok(self.segment_events(segment, &analysis))
    }

    fn finish(&mut self, _last_segment_index: Option<u64>) -> Result<Vec<AnalysisEvent>> {
        self.document_events()
    }
}

fn build_topic_model(
    sentences: &[Sentence],
    tokens: &[Token],
    lemmas: &LemmatizationResult,
    chunks: &[PhraseChunk],
    language: Option<&LanguagePrediction>,
) -> TopicModel {
    let stop_words = LexiconStore::default()
        .stop_words_for(
            language
                .map(|prediction| prediction.language.as_str())
                .unwrap_or("en"),
        )
        .map(|lexicon| lexicon.entries.clone())
        .unwrap_or_default();

    let mut lemma_counts = BTreeMap::<String, usize>::new();
    for lemma in &lemmas.lemmas {
        let token = &tokens[lemma.token_index];
        if matches!(token.kind, TokenKind::Word | TokenKind::Number)
            && lemma.value.chars().count() >= 3
            && !stop_words.contains(&lemma.value)
        {
            *lemma_counts.entry(lemma.value.clone()).or_insert(0) += 1;
        }
    }

    let mut ranked_terms = lemma_counts.into_iter().collect::<Vec<_>>();
    ranked_terms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let descriptors = ranked_terms
        .into_iter()
        .take(3)
        .map(|(term, count)| TopicDescriptor {
            label: term.clone(),
            terms: vec![term],
            score: count as f32 / sentences.len().max(1) as f32,
        })
        .collect::<Vec<_>>();
    let clusters = descriptors
        .iter()
        .enumerate()
        .map(|(index, descriptor)| {
            let sentence_indices = chunks
                .iter()
                .filter(|chunk| {
                    chunk.kind == ChunkKind::NounPhrase
                        && descriptor
                            .terms
                            .iter()
                            .any(|term| chunk.text.to_lowercase().contains(term))
                })
                .map(|chunk| chunk.sentence_index)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            TopicCluster {
                id: format!("topic-{index}"),
                descriptor: descriptor.clone(),
                sentence_indices,
            }
        })
        .collect();
    TopicModel {
        descriptors,
        clusters,
    }
}

fn build_style_profile(
    text: &str,
    sentences: &[Sentence],
    tokens: &[Token],
    lemmas: &[Lemma],
    dependencies: &[DependencyTree],
) -> StyleProfile {
    let question_count = text.matches('?').count();
    let exclamation_count = text.matches('!').count();
    let sentence_token_ranges = sentence_token_ranges(sentences, tokens);
    let sentence_lengths = sentence_token_ranges
        .iter()
        .map(|(start, end)| end.saturating_sub(*start))
        .collect::<Vec<_>>();
    let average_sentence_tokens = if sentence_lengths.is_empty() {
        0.0
    } else {
        sentence_lengths.iter().sum::<usize>() as f32 / sentence_lengths.len() as f32
    };
    let clause_markers = tokens
        .iter()
        .filter(|token| {
            matches!(
                token.normalized.as_str(),
                "and" | "but" | "because" | "that" | "which"
            )
        })
        .count();
    let clause_density = if sentences.is_empty() {
        0.0
    } else {
        clause_markers as f32 / sentences.len() as f32
    };
    let lexical_tokens = tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Word | TokenKind::Number))
        .collect::<Vec<_>>();
    let unique_tokens = lexical_tokens
        .iter()
        .map(|token| token.normalized.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let unique_lemmas = lemmas
        .iter()
        .map(|lemma| lemma.value.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let type_token_ratio = if lexical_tokens.is_empty() {
        0.0
    } else {
        unique_tokens as f32 / lexical_tokens.len() as f32
    };
    let lemma_type_token_ratio = if lemmas.is_empty() {
        0.0
    } else {
        unique_lemmas as f32 / lemmas.len() as f32
    };
    let disfluency_markers = lexical_tokens
        .iter()
        .filter(|token| matches!(token.normalized.as_str(), "uh" | "um" | "er" | "ah" | "hmm"))
        .count();
    let disfluency_rate = if lexical_tokens.is_empty() {
        0.0
    } else {
        disfluency_markers as f32 / lexical_tokens.len() as f32
    };
    let passive_voice_hits = dependencies
        .iter()
        .filter(|tree| {
            tree.root_token_index.is_some_and(|root| {
                tokens[root].normalized.ends_with("ed")
                    && tree.edges.iter().any(|edge| {
                        edge.relation == DependencyRelation::Aux
                            && matches!(
                                tokens[edge.dependent_token_index].normalized.as_str(),
                                "was" | "were" | "is" | "been"
                            )
                    })
            })
        })
        .count();
    let passive_voice_estimate = if dependencies.is_empty() {
        0.0
    } else {
        passive_voice_hits as f32 / dependencies.len() as f32
    };
    let contractions = lexical_tokens
        .iter()
        .filter(|token| token.text.contains('\''))
        .count();
    let technical_terms = lexical_tokens
        .iter()
        .filter(|token| {
            matches!(
                token.normalized.as_str(),
                "api" | "schema" | "tokenizer" | "pipeline"
            )
        })
        .count();
    let formality_score = ((average_sentence_tokens / 20.0) + (technical_terms as f32 / 10.0)
        - (contractions as f32 / lexical_tokens.len().max(1) as f32))
        .clamp(0.0, 1.0);
    let repetitiveness = 1.0 - type_token_ratio;
    let register = if technical_terms >= 2 {
        RegisterEstimate::Technical
    } else if contractions >= 2 {
        RegisterEstimate::Conversational
    } else if formality_score > 0.65 {
        RegisterEstimate::Formal
    } else if formality_score < 0.25 {
        RegisterEstimate::Informal
    } else {
        RegisterEstimate::Neutral
    };

    StyleProfile {
        register,
        complexity: ComplexityMetrics {
            average_sentence_tokens,
            clause_density,
            type_token_ratio,
            lemma_type_token_ratio,
            disfluency_rate,
        },
        question_count,
        exclamation_count,
        passive_voice_estimate,
        formality_score,
        repetitiveness,
        disfluency_markers,
    }
}

fn heuristic_lemma(token: &Token, language: Option<&str>, options: &LemmaOptions) -> (String, f32) {
    let normalized = if token.kind == TokenKind::Punctuation {
        return (token.text.clone(), 1.0);
    } else if matches!(
        token.kind,
        TokenKind::Url
            | TokenKind::Email
            | TokenKind::Mention
            | TokenKind::Hashtag
            | TokenKind::Number
    ) {
        return (token.normalized.clone(), 1.0);
    } else if options.lowercase_proper_nouns {
        token.text.to_lowercase()
    } else {
        token.normalized.clone()
    };

    if language != Some("en") {
        return (normalized, 0.35);
    }

    let irregular = BTreeMap::from([
        ("was", "be"),
        ("were", "be"),
        ("is", "be"),
        ("are", "be"),
        ("am", "be"),
        ("has", "have"),
        ("had", "have"),
        ("did", "do"),
        ("does", "do"),
        ("went", "go"),
        ("gone", "go"),
        ("children", "child"),
        ("men", "man"),
        ("women", "woman"),
        ("mice", "mouse"),
        ("geese", "goose"),
    ]);
    if let Some(lemma) = irregular.get(normalized.as_str()) {
        return ((*lemma).to_string(), 0.95);
    }
    if normalized.ends_with("ies") && normalized.len() > 4 {
        return (format!("{}y", &normalized[..normalized.len() - 3]), 0.85);
    }
    if normalized.ends_with("ing") && normalized.len() > 5 {
        return (
            normalized
                .trim_end_matches("ing")
                .trim_end_matches(char::is_whitespace)
                .to_string(),
            0.8,
        );
    }
    if normalized.ends_with("ed") && normalized.len() > 4 {
        return (normalized.trim_end_matches("ed").to_string(), 0.8);
    }
    if normalized.ends_with("es") && normalized.len() > 4 {
        let candidate = normalized.trim_end_matches("es");
        if candidate.ends_with('h')
            || candidate.ends_with('s')
            || candidate.ends_with('x')
            || candidate.ends_with('z')
            || candidate.ends_with("ch")
            || candidate.ends_with("sh")
        {
            return (candidate.to_string(), 0.78);
        }
    }
    if normalized.ends_with('s') && normalized.len() > 3 && !normalized.ends_with("ss") {
        return (normalized.trim_end_matches('s').to_string(), 0.75);
    }
    (normalized, 0.7)
}

fn heuristic_pos(
    token: &Token,
    lemma: Option<&str>,
    token_index: usize,
    is_sentence_initial: bool,
) -> PosAnnotation {
    let normalized = token.normalized.as_str();
    let lemma = lemma.unwrap_or(normalized);
    let (tag, confidence, reason) = match token.kind {
        TokenKind::Punctuation => (PosTag::Punct, 1.0, "punctuation token".to_string()),
        TokenKind::Number => (PosTag::Num, 1.0, "numeric token".to_string()),
        TokenKind::Url | TokenKind::Email => (PosTag::Sym, 1.0, "symbolic token".to_string()),
        TokenKind::Mention | TokenKind::Hashtag => (PosTag::Propn, 0.9, "social token".to_string()),
        TokenKind::Other => (PosTag::X, 0.5, "unclassified token".to_string()),
        TokenKind::Word => {
            if ["the", "a", "an", "this", "that", "these", "those"].contains(&normalized) {
                (PosTag::Det, 0.95, "determiner lexicon".to_string())
            } else if [
                "he", "she", "they", "it", "we", "i", "you", "me", "us", "them",
            ]
            .contains(&normalized)
            {
                (PosTag::Pron, 0.95, "pronoun lexicon".to_string())
            } else if ["and", "or", "but"].contains(&normalized) {
                (PosTag::Cconj, 0.95, "coordinating conjunction".to_string())
            } else if ["because", "if", "while", "although", "that"].contains(&normalized) {
                (PosTag::Sconj, 0.9, "subordinating conjunction".to_string())
            } else if [
                "in", "on", "at", "for", "from", "to", "with", "by", "over", "under",
            ]
            .contains(&normalized)
            {
                (PosTag::Adp, 0.95, "adposition lexicon".to_string())
            } else if [
                "is", "are", "was", "were", "be", "been", "being", "am", "have", "has", "had",
                "do", "does", "did", "will", "shall",
            ]
            .contains(&normalized)
            {
                (PosTag::Aux, 0.9, "auxiliary lexicon".to_string())
            } else if normalized.ends_with("ly") {
                (PosTag::Adv, 0.8, "adverb suffix".to_string())
            } else if normalized.ends_with("ing")
                || normalized.ends_with("ed")
                || normalized.ends_with("es")
                || ["go", "make", "say", "present", "launch", "visit"].contains(&lemma)
            {
                (PosTag::Verb, 0.8, "verbal morphology".to_string())
            } else if normalized.ends_with("ous")
                || normalized.ends_with("ive")
                || normalized.ends_with("al")
                || normalized.ends_with("ful")
            {
                (PosTag::Adj, 0.75, "adjective suffix".to_string())
            } else if token
                .text
                .chars()
                .next()
                .is_some_and(|ch| ch.is_uppercase())
                && ![
                    "the", "a", "an", "this", "that", "these", "those", "and", "but", "or", "in",
                    "on", "at", "for", "from", "to", "with", "by",
                ]
                .contains(&normalized)
                && (!is_sentence_initial || token.text.chars().skip(1).any(|ch| ch.is_lowercase()))
            {
                (PosTag::Propn, 0.7, "capitalized token".to_string())
            } else {
                (PosTag::Noun, 0.65, "noun fallback".to_string())
            }
        }
    };
    PosAnnotation {
        token_index,
        tag,
        confidence,
        reason,
    }
}

fn sentence_token_ranges(sentences: &[Sentence], tokens: &[Token]) -> Vec<(usize, usize)> {
    sentences
        .iter()
        .map(|sentence| {
            let start = tokens
                .iter()
                .position(|token| token.span.byte_start >= sentence.span.byte_start)
                .unwrap_or(tokens.len());
            let end = tokens[start..]
                .iter()
                .position(|token| token.span.byte_end > sentence.span.byte_end)
                .map(|offset| start + offset)
                .unwrap_or(tokens.len());
            (start, end.max(start))
        })
        .collect()
}

fn spans_overlap(left: TextSpan, right: TextSpan) -> bool {
    left.byte_start < right.byte_end && right.byte_start < left.byte_end
}

fn byte_span_to_text_span(text: &str, byte_start: usize, byte_end: usize) -> Option<TextSpan> {
    if byte_start > byte_end || byte_end > text.len() {
        return None;
    }
    let char_start = text[..byte_start].chars().count();
    let char_end = text[..byte_end].chars().count();
    Some(TextSpan {
        byte_start,
        byte_end,
        char_start,
        char_end,
    })
}

fn script_default_language(script: &str) -> Option<&'static str> {
    match script {
        "Han" | "Hiragana" | "Katakana" => Some("ja"),
        "Hangul" => Some("ko"),
        "Arabic" => Some("ar"),
        "Cyrillic" => Some("ru"),
        "Hebrew" => Some("he"),
        "Greek" => Some("el"),
        "Devanagari" => Some("hi"),
        "Latin" => Some("en"),
        _ => None,
    }
}

fn nearest_following(
    _tokens: &[Token],
    pos: &[PosAnnotation],
    token_index: usize,
    sentence_end: usize,
    matcher: impl Fn(PosTag) -> bool,
) -> Option<usize> {
    ((token_index + 1)..sentence_end).find(|index| {
        pos.get(*index)
            .map(|annotation| matcher(annotation.tag))
            .unwrap_or(false)
    })
}

fn is_date_token(token: &Token) -> bool {
    let normalized = token.normalized.as_str();
    normalized.chars().any(|ch| ch.is_ascii_digit())
        || matches!(
            normalized,
            "january"
                | "february"
                | "march"
                | "april"
                | "may"
                | "june"
                | "july"
                | "august"
                | "september"
                | "october"
                | "november"
                | "december"
        )
}

fn classify_capitalized_entity(tokens: &[Token]) -> EntityType {
    let normalized = tokens
        .iter()
        .map(|token| token.normalized.as_str())
        .collect::<Vec<_>>();
    if normalized
        .iter()
        .any(|token| matches!(*token, "inc" | "corp" | "ltd" | "gmbh" | "ag"))
    {
        EntityType::Organization
    } else if normalized.iter().any(|token| {
        is_date_token(&Token {
            text: (*token).to_string(),
            normalized: (*token).to_string(),
            span: TextSpan {
                byte_start: 0,
                byte_end: token.len(),
                char_start: 0,
                char_end: token.chars().count(),
            },
            kind: TokenKind::Word,
        })
    }) {
        EntityType::Date
    } else if tokens.len() == 1 {
        EntityType::Person
    } else {
        EntityType::Organization
    }
}

fn build_entity(
    index: usize,
    entity_type: EntityType,
    text: &str,
    sentence_index: usize,
    token_range: std::ops::Range<usize>,
    tokens: &[Token],
    confidence: f32,
) -> NamedEntity {
    let token_start = token_range.start;
    let token_end = token_range.end;
    let span = TextSpan {
        byte_start: tokens[token_start].span.byte_start,
        byte_end: tokens[token_end - 1].span.byte_end,
        char_start: tokens[token_start].span.char_start,
        char_end: tokens[token_end - 1].span.char_end,
    };
    let mention_text = text[span.byte_start..span.byte_end].to_string();
    NamedEntity {
        id: format!("mention-{index}"),
        entity_type,
        mention: EntityMentionSpan {
            text: mention_text.clone(),
            span,
        },
        normalized: mention_text.to_lowercase(),
        sentence_index,
        token_start,
        token_end,
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_analysis_transcription::{parse_srt, parse_webvtt};

    #[test]
    fn detects_english_text() {
        let detector = LanguageDetector::default();
        let profile = detector.detect_text("This is a simple English sentence with the roadmap.");
        assert_eq!(profile.primary.unwrap().language, "en");
    }

    #[test]
    fn detects_mixed_script_text() {
        let detector = LanguageDetector::default();
        let profile = detector.detect_text("Hello 東京 and Berlin");
        assert!(profile.is_mixed || profile.dominant_script.is_some());
    }

    #[test]
    fn falls_back_to_und_for_empty_text() {
        let detector = LanguageDetector::default();
        let profile = detector.detect_text("");
        let primary = profile
            .primary
            .expect("empty text still yields a fallback profile");

        assert_eq!(primary.language, "und");
        assert_eq!(primary.confidence, 0.0);
        assert_eq!(primary.reason, "script fallback");
        assert_eq!(profile.token_count, 0);
        assert!(profile.alternatives.is_empty());
        assert!(profile.sentence_predictions.is_empty());
    }

    #[test]
    fn selects_default_mixed_tokenizer_policy() {
        let registry = TokenizerRegistry::default();
        let selection = registry.select(Some("en"), Some("linguistic-analysis"), None);
        assert_eq!(selection.mode, TokenizationMode::Mixed);
        assert!(selection.source.is_some());
    }

    #[test]
    fn tokenizer_selection_prefers_task_override_and_word_mode_has_no_source() {
        let mut registry = TokenizerRegistry::default();
        let language_source = TokenizerSource::local("/tmp/language-tokenizer.json");
        let family_source = TokenizerSource::local("/tmp/family-tokenizer.json");
        let task_source = TokenizerSource::local("/tmp/task-tokenizer.json");

        registry
            .policy
            .language_overrides
            .insert("en".to_string(), language_source);
        registry
            .policy
            .model_family_overrides
            .insert("bert".to_string(), family_source);
        registry
            .policy
            .task_overrides
            .insert("classification".to_string(), task_source.clone());

        let selection = registry.select(Some("en"), Some("classification"), Some("bert"));
        assert_eq!(selection.source, Some(task_source));
        assert_eq!(selection.reason, "task override for `classification`");

        registry.policy.mode = TokenizationMode::Word;
        let word_only = registry.select(Some("en"), Some("classification"), Some("bert"));
        assert_eq!(word_only.mode, TokenizationMode::Word);
        assert_eq!(word_only.source, None);
    }

    #[test]
    fn aligns_surface_tokens_to_fake_subwords() {
        let text = "don't panic";
        let tokens = tokenize(
            text,
            &TextProcessingOptions {
                include_punctuation: false,
                ..TextProcessingOptions::default()
            },
        );
        let alignment = align_tokenized_text(
            text,
            &tokens,
            TokenizerSelection {
                mode: TokenizationMode::Mixed,
                source: Some(TokenizerSource::default()),
                language: Some("en".to_string()),
                task: None,
                model_family: None,
                reason: "test".to_string(),
            },
            &TokenizedText {
                input_ids: vec![1, 2, 3, 4],
                attention_mask: vec![1, 1, 1, 1],
                token_type_ids: Some(vec![0, 0, 0, 0]),
                offsets: vec![Some((0, 2)), Some((2, 5)), Some((6, 8)), Some((8, 11))],
            },
        )
        .unwrap();
        assert_eq!(alignment.aligned_tokens.len(), 2);
        assert_eq!(alignment.aligned_tokens[0].subword_indices, vec![0, 1]);
    }

    #[test]
    fn ignores_invalid_subword_offsets_during_alignment() {
        let text = "hello world";
        let tokens = tokenize(
            text,
            &TextProcessingOptions {
                include_punctuation: false,
                ..TextProcessingOptions::default()
            },
        );
        let alignment = align_tokenized_text(
            text,
            &tokens,
            TokenizerSelection {
                mode: TokenizationMode::Mixed,
                source: Some(TokenizerSource::default()),
                language: Some("en".to_string()),
                task: None,
                model_family: None,
                reason: "test".to_string(),
            },
            &TokenizedText {
                input_ids: vec![10, 11, 12, 13],
                attention_mask: vec![1, 1, 1, 1],
                token_type_ids: None,
                offsets: vec![Some((0, 5)), Some((5, 5)), Some((6, 11)), Some((50, 51))],
            },
        )
        .unwrap();

        assert_eq!(alignment.subwords[0].text.as_deref(), Some("hello"));
        assert_eq!(alignment.subwords[1].span, None);
        assert_eq!(alignment.subwords[2].text.as_deref(), Some("world"));
        assert_eq!(alignment.subwords[3].span, None);
        assert_eq!(alignment.aligned_tokens[0].subword_indices, vec![0]);
        assert_eq!(alignment.aligned_tokens[1].subword_indices, vec![2]);
    }

    #[test]
    fn lemmatizes_plural_and_inflected_tokens() {
        let tokens = tokenize(
            "Cars were running",
            &TextProcessingOptions {
                include_punctuation: false,
                ..TextProcessingOptions::default()
            },
        );
        let lemmas = lemmatize_tokens(&tokens, Some("en"), &LemmaOptions::default());
        assert_eq!(lemmas.lemmas[0].value, "car");
        assert_eq!(lemmas.lemmas[1].value, "be");
    }

    #[test]
    fn morphology_annotation_tolerates_missing_lemma_and_pos_entries() {
        let tokens = tokenize(
            "They were testing robots",
            &TextProcessingOptions {
                include_punctuation: false,
                ..TextProcessingOptions::default()
            },
        );
        let lemmas = lemmatize_tokens(&tokens, Some("en"), &LemmaOptions::default());
        let mut partial_lemmas = lemmas.clone();
        partial_lemmas.lemmas.truncate(3);
        let pos = PosTagger::default().tag_tokens(&tokens, &lemmas);
        let annotations = annotate_morphology(&tokens, &partial_lemmas, &pos[..3]);

        assert_eq!(annotations.len(), 4);
        assert!(annotations[0]
            .tags
            .features
            .contains(&MorphFeature::Person3));
        assert!(annotations[0]
            .tags
            .features
            .contains(&MorphFeature::NumberPlur));
        assert!(annotations[1]
            .tags
            .features
            .contains(&MorphFeature::TensePast));
        assert_eq!(annotations[3].lemma, None);
        assert!(annotations[3].tags.features.is_empty());
    }

    #[test]
    fn tags_pos_and_chunks_simple_sentence() {
        let text = "The new product launches today";
        let tokens = tokenize(
            text,
            &TextProcessingOptions {
                include_punctuation: true,
                ..TextProcessingOptions::default()
            },
        );
        let lemmas = lemmatize_tokens(&tokens, Some("en"), &LemmaOptions::default());
        let tagger = PosTagger::default();
        let pos = tagger.tag_tokens(&tokens, &lemmas);
        assert!(pos.iter().any(|annotation| annotation.tag == PosTag::Verb));
        let sentences = split_sentence_spans(text, &TextProcessingOptions::default());
        let chunks = chunk_phrases(text, &sentences, &tokens, &pos);
        assert!(chunks
            .iter()
            .any(|chunk| chunk.kind == ChunkKind::NounPhrase));
        assert!(chunks
            .iter()
            .any(|chunk| chunk.kind == ChunkKind::VerbPhrase));
    }

    #[test]
    fn builds_dependency_tree_with_subject_and_object_like_relations() {
        let text = "Alice launched product";
        let tokens = tokenize(
            text,
            &TextProcessingOptions {
                include_punctuation: true,
                ..TextProcessingOptions::default()
            },
        );
        let lemmas = lemmatize_tokens(&tokens, Some("en"), &LemmaOptions::default());
        let mut pos = PosTagger::default().tag_tokens(&tokens, &lemmas);
        pos[0].tag = PosTag::Propn;
        pos[1].tag = PosTag::Verb;
        pos[2].tag = PosTag::Noun;
        let trees = DependencyParser.parse_document(
            &split_sentence_spans(text, &TextProcessingOptions::default()),
            &tokens,
            &pos,
        );
        assert_eq!(trees.len(), 1);
        assert!(trees[0]
            .edges
            .iter()
            .any(|edge| edge.relation == DependencyRelation::Nsubj));
    }

    #[test]
    fn extracts_entities_coreference_and_events() {
        let text = "Alice visited Berlin. She presented the roadmap.";
        let analysis = analyze_text(text, &LinguisticAnalysisOptions::default()).unwrap();
        assert!(analysis
            .entities
            .iter()
            .any(|entity| entity.entity_type == EntityType::Person));
        assert!(!analysis.coreference.is_empty());
        assert!(!analysis.events.is_empty());
    }

    #[test]
    fn analyzes_subtitle_segments_per_cue_and_in_aggregate() {
        let cues = vec![
            TranscriptSegment {
                index: 0,
                start_seconds: Some(0.0),
                end_seconds: Some(1.0),
                text: "Alice visited Berlin".to_string(),
                language: Some("en".to_string()),
                speaker: Some("narrator".to_string()),
                confidence: Some(0.9),
                is_final: true,
            },
            TranscriptSegment {
                index: 1,
                start_seconds: Some(1.0),
                end_seconds: Some(2.0),
                text: "Sie praesentierte die Roadmap".to_string(),
                language: Some("de".to_string()),
                speaker: None,
                confidence: Some(0.8),
                is_final: true,
            },
        ];

        let analysis =
            analyze_subtitle_segments(&cues, &LinguisticAnalysisOptions::default()).unwrap();

        assert_eq!(analysis.cues.len(), 2);
        assert_eq!(analysis.cues[0].cue, cues[0]);
        assert_eq!(analysis.cues[1].cue, cues[1]);
        assert!(!analysis.cues[0].analysis.tokens.is_empty());
        assert!(!analysis.cues[1].analysis.tokens.is_empty());
        assert!(!analysis.aggregate.tokens.is_empty());
    }

    #[test]
    fn analyzes_transcription_using_explicit_transcript_text_when_present() {
        let transcription = parse_srt(
            "1\n00:00:00,000 --> 00:00:01,000\nAlice visited Berlin\n\n2\n00:00:01,000 --> 00:00:02,000\nShe presented the roadmap\n",
        )
        .unwrap();

        let analysis =
            analyze_transcription(&transcription, &LinguisticAnalysisOptions::default()).unwrap();

        assert_eq!(analysis.cues.len(), 2);
        assert!(!analysis.aggregate.entities.is_empty());
        assert!(analysis
            .aggregate
            .tokens
            .iter()
            .any(|token| token.normalized == "roadmap"));
    }

    #[test]
    fn analyzes_transcription_falling_back_to_joined_cue_text() {
        let mut transcription = parse_webvtt(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello Berlin\n\n00:00:01.000 --> 00:00:02.000\nHola Madrid\n",
        )
        .unwrap();
        transcription.text = Some("   ".to_string());

        let analysis =
            analyze_transcription(&transcription, &LinguisticAnalysisOptions::default()).unwrap();

        assert_eq!(analysis.cues.len(), 2);
        assert!(analysis
            .aggregate
            .tokens
            .iter()
            .any(|token| token.normalized == "hello"));
        assert!(analysis
            .aggregate
            .tokens
            .iter()
            .any(|token| token.normalized == "hola"));
    }

    #[test]
    fn classifies_discourse_topics_and_style() {
        let text = "First, we introduce the API. However, the migration is tricky. Finally, the rollout is stable.";
        let analysis = analyze_text(text, &LinguisticAnalysisOptions::default()).unwrap();
        assert!(!analysis.discourse.is_empty());
        assert!(!analysis.topics.descriptors.is_empty());
        assert!(analysis.style.complexity.average_sentence_tokens > 0.0);
    }

    #[test]
    fn analyzer_emits_segment_and_document_events() {
        let mut analyzer = LinguisticAnalyzer::new(LinguisticAnalysisOptions::default());
        let segment = OwnedTextSegment::new(0, "Alice presented the roadmap");
        let events = analyzer.process_segment(&segment.as_segment()).unwrap();
        assert!(events
            .iter()
            .any(|event| event.label.starts_with("text:language:")));
        let final_events = analyzer.finish(Some(0)).unwrap();
        assert!(final_events
            .iter()
            .any(|event| event.label.starts_with("text:topic:")));
    }

    #[test]
    fn extracts_dates_and_amounts_without_pos_annotations() {
        let text = "Launch on January 2024 costs $99.";
        let sentences = vec![Sentence {
            text: text.to_string(),
            span: TextSpan {
                byte_start: 0,
                byte_end: text.len(),
                char_start: 0,
                char_end: text.chars().count(),
            },
            token_count: 6,
        }];
        let tokens = vec![
            Token {
                text: "Launch".to_string(),
                normalized: "launch".to_string(),
                span: TextSpan {
                    byte_start: 0,
                    byte_end: 6,
                    char_start: 0,
                    char_end: 6,
                },
                kind: TokenKind::Word,
            },
            Token {
                text: "on".to_string(),
                normalized: "on".to_string(),
                span: TextSpan {
                    byte_start: 7,
                    byte_end: 9,
                    char_start: 7,
                    char_end: 9,
                },
                kind: TokenKind::Word,
            },
            Token {
                text: "January".to_string(),
                normalized: "january".to_string(),
                span: TextSpan {
                    byte_start: 10,
                    byte_end: 17,
                    char_start: 10,
                    char_end: 17,
                },
                kind: TokenKind::Word,
            },
            Token {
                text: "2024".to_string(),
                normalized: "2024".to_string(),
                span: TextSpan {
                    byte_start: 18,
                    byte_end: 22,
                    char_start: 18,
                    char_end: 22,
                },
                kind: TokenKind::Number,
            },
            Token {
                text: "costs".to_string(),
                normalized: "costs".to_string(),
                span: TextSpan {
                    byte_start: 23,
                    byte_end: 28,
                    char_start: 23,
                    char_end: 28,
                },
                kind: TokenKind::Word,
            },
            Token {
                text: "$99".to_string(),
                normalized: "$99".to_string(),
                span: TextSpan {
                    byte_start: 29,
                    byte_end: 32,
                    char_start: 29,
                    char_end: 32,
                },
                kind: TokenKind::Other,
            },
        ];
        let entities = extract_named_entities(text, &sentences, &tokens, &[]);

        assert!(entities
            .iter()
            .any(|entity| entity.entity_type == EntityType::Date
                && entity.mention.text.to_lowercase().contains("january")));
        assert!(entities
            .iter()
            .any(|entity| entity.entity_type == EntityType::Date
                && entity.mention.text.contains("2024")));
        assert!(
            entities
                .iter()
                .any(|entity| entity.entity_type == EntityType::Amount
                    && entity.mention.text == "$99")
        );
    }

    #[test]
    fn text_nlp_pipeline_exposes_rich_graph_and_profile_metadata() {
        let pipeline = TextNlpPipeline::default();
        let analysis = pipeline
            .analyze_text("Alice presented the roadmap in Berlin.")
            .unwrap();

        assert_eq!(analysis.profile, AnalysisProfile::Rich);
        assert_eq!(analysis.graph.tokens.len(), analysis.tokens.len());
        assert_eq!(analysis.provenance, AnnotationProvenance::Tokenizer);
        assert!(analysis.confidence.get() > 0.0);
        assert_eq!(analysis.token_ref(0).unwrap().text, "Alice");
    }

    #[test]
    fn fast_profile_disables_heavier_annotations() {
        let pipeline = TextNlpPipeline::new(TextNlpConfig::fast());
        let analysis = pipeline
            .analyze_text("Alice presented the roadmap in Berlin.")
            .unwrap();

        assert_eq!(analysis.profile, AnalysisProfile::Fast);
        assert!(analysis.alignments.is_none());
        assert!(analysis.events.is_empty());
        assert!(analysis.discourse.is_empty());
        assert!(analysis.topics.descriptors.is_empty());
    }
}
