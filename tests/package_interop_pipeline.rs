use video_analysis as va;

fn timestamp(seconds: i64) -> va::Timestamp {
    va::Timestamp::new(seconds, va::Timebase::new(1, 1))
}

fn position(frame_index: u64) -> va::FramePosition {
    va::FramePosition {
        frame_index,
        timestamp: timestamp(frame_index as i64),
    }
}

fn dense_point_from_vector(
    id: impl Into<String>,
    vector: &va::vector_core::DenseVector,
) -> va::dense::DensePoint {
    let coordinates = vector
        .as_slice()
        .iter()
        .take(8)
        .map(|value| *value as f64)
        .collect::<Vec<_>>();
    va::dense::DensePoint::new(coordinates).unwrap().named(id)
}

#[test]
fn transcript_to_features_dataset_pipeline_keeps_packages_compatible() {
    let segments = vec![
        va::OwnedTextSegment::new(
            0,
            "Rust crates stream audio, transcript, and embeddings through stable data pipelines.",
        )
        .timestamp(timestamp(0))
        .language("en"),
        va::OwnedTextSegment::new(
            1,
            "Semantic search compares transcript embeddings with vector indexes and dense summaries.",
        )
        .timestamp(timestamp(1))
        .language("en"),
        va::OwnedTextSegment::new(
            2,
            "Video features join scenes, observations, and text events before reports are written.",
        )
        .timestamp(timestamp(2))
        .language("en"),
    ];

    let documents = segments
        .iter()
        .map(|segment| {
            va::text_core::OwnedTextDocument::from_segment(
                format!("segment:{}", segment.segment_index),
                segment,
            )
        })
        .collect::<Vec<_>>();

    let mut text_pipeline = va::TextPipeline::builder()
        .analyzer(va::text_features::TextStatsAnalyzer)
        .analyzer(va::text_features::KeywordAnalyzer::default())
        .analyzer(va::text_features::PatternAnalyzer)
        .build()
        .unwrap();

    let mut text_events = Vec::new();
    for segment in segments.iter().cloned() {
        text_events.extend(text_pipeline.process_segment(segment).unwrap().events);
    }
    let text_result = text_pipeline.finish_analysis().unwrap();
    assert_eq!(text_result.segments_processed, segments.len() as u64);
    assert!(text_events
        .iter()
        .any(|event| event.label == "text:keyword:rust"));

    let corpus = va::text_corpus::TfIdfCorpus::from_texts(
        documents.iter().map(|document| document.text.as_str()),
        va::text_corpus::CorpusOptions::default(),
    )
    .unwrap();
    assert_eq!(corpus.documents()[0].id, "doc-0");
    assert_eq!(corpus.documents()[1].id, "doc-1");
    assert_eq!(corpus.documents()[2].id, "doc-2");
    assert_eq!(corpus.stats().documents, documents.len());
    assert!(corpus
        .document_tfidf("doc-0", 5)
        .unwrap()
        .iter()
        .any(|term| term.term == "rust"));
    assert_eq!(
        corpus.search("video scene reports", 1).unwrap()[0].id,
        "doc-2"
    );

    let embedder = va::text_semantics::HashedTextEmbedder::new(
        va::text_semantics::TextEmbeddingConfig {
            dimensions: 64,
            use_idf: true,
        },
        va::text_corpus::CorpusOptions::default(),
    )
    .unwrap();
    let mut semantic_index = va::text_semantics::SemanticTextIndex::new(embedder.clone());
    for document in &documents {
        semantic_index
            .add_text_document(&document.as_document())
            .unwrap();
    }
    let semantic_matches = semantic_index
        .search(documents[0].text.as_str(), 1)
        .unwrap();
    assert_eq!(semantic_matches[0].id, "segment:0");

    let vectors = documents
        .iter()
        .map(|document| {
            embedder
                .embed_text_with_corpus(document.text.as_str(), Some(semantic_index.corpus()))
                .unwrap()
        })
        .collect::<Vec<_>>();
    let mut vector_index = va::vector_index::VectorSearchIndex::new();
    for (document, vector) in documents.iter().zip(&vectors) {
        vector_index
            .add(va::vector_index::VectorRecord::new(
                document.id.clone(),
                vector.clone(),
            ))
            .unwrap();
    }
    let nearest = vector_index
        .search(&vectors[0], va::vector_index::SearchConfig::default())
        .unwrap();
    assert_eq!(nearest[0].id, "segment:0");

    let dense_dataset = va::dense::DenseDataset::from_points(
        documents
            .iter()
            .zip(&vectors)
            .map(|(document, vector)| dense_point_from_vector(document.id.clone(), vector)),
    )
    .unwrap();
    assert_eq!(dense_dataset.len(), documents.len());
    assert_eq!(
        dense_dataset.averages().unwrap().count,
        documents.len() as u64
    );
    assert!(!dense_dataset
        .buckets(&va::dense::BucketGrid::uniform(8, 1.0).unwrap())
        .unwrap()
        .is_empty());

    let first_vector = vectors[0].as_slice().to_vec();
    let mut bucket_aggregator = va::data::BucketAggregator::new(
        va::data::BucketConfig::record_count(2)
            .unwrap()
            .max_vector_dimensions(64),
    )
    .unwrap();
    assert!(bucket_aggregator
        .push(va::data::DataRecord::text_segment(
            "transcript",
            &segments[0].as_segment(),
        ))
        .unwrap()
        .is_empty());
    let completed_buckets = bucket_aggregator
        .push(va::data::DataRecord::vector(
            "semantic_embedding",
            0,
            Some(timestamp(0)),
            &first_vector,
        ))
        .unwrap();
    let bucket = &completed_buckets[0];
    assert_eq!(bucket.streams["transcript"].text.segments, 1);
    assert_eq!(
        bucket.streams["semantic_embedding"].vector.dimensions,
        Some(64)
    );

    let scene = va::Scene {
        start: position(0),
        end: position(3),
    };
    let mut dataset = va::dataset_records::AnalysisDataset::empty();
    dataset.push(va::dataset_records::DatasetRecord::Scene(
        va::dataset_records::SceneRecord::from_scene(0, &scene),
    ));
    for segment in &segments {
        dataset.push(va::dataset_records::DatasetRecord::TextSegment(
            va::dataset_records::TextSegmentRecord::from_segment(
                "transcript",
                &segment.as_segment(),
            ),
        ));
    }
    dataset.extend_events(text_events.clone());
    dataset.extend_observations(segments.iter().map(|segment| {
        va::Observation::new("transcript", va::ObservationKind::Text)
            .at_timestamp(segment.timestamp.unwrap())
            .in_scene(0)
            .label("transcript")
            .text(segment.text.clone())
            .attribute("language", "en")
    }));
    dataset.push(va::dataset_records::DatasetRecord::Feature(
        va::dataset_records::FeatureRecord::new(
            "text.embedding",
            va::dataset_records::FeatureValue::Vector(first_vector),
        )
        .scope("global")
        .timestamp(timestamp(0)),
    ));
    dataset.push(va::dataset_records::DatasetRecord::Feature(
        va::dataset_records::FeatureRecord::number(
            "semantic.match_score",
            semantic_matches[0].score as f64,
        )
        .scope("global")
        .timestamp(timestamp(0)),
    ));

    let mut sorted_records = dataset.records.clone();
    va::transform::sort_by_time(&mut sorted_records);
    assert_eq!(
        va::transform::record_timestamp_seconds(&sorted_records[0]),
        Some(0.0)
    );
    assert_eq!(
        va::transform::dedupe_records(
            dataset
                .records
                .iter()
                .cloned()
                .chain(dataset.records.iter().cloned())
                .collect()
        )
        .len(),
        dataset.records.len()
    );
    assert_eq!(
        va::transform::group_by_scene(&dataset)[0].scene.scene_index,
        0
    );
    assert!(
        va::transform::window_by_time(&dataset.records, 1.5)
            .unwrap()
            .len()
            >= 2
    );

    let feature_pipeline = va::features::FeaturePipeline::builder()
        .extractor(va::features::SceneStatsExtractor)
        .extractor(va::features::TranscriptStatsExtractor)
        .extractor(va::features::ObservationLabelHistogramExtractor::default())
        .extractor(va::features::FeatureVectorMeanExtractor)
        .build();
    let feature_records = feature_pipeline.extract(&dataset).unwrap();
    assert!(feature_records
        .iter()
        .any(|feature| feature.name == "scene.text_observations"));
    assert!(feature_records
        .iter()
        .any(|feature| feature.name == "transcript.word_count"));
    assert!(feature_records
        .iter()
        .any(|feature| feature.name == "text.embedding.mean"));

    let numeric_features = dataset.features().cloned().collect::<Vec<_>>();
    let resampled = va::transform::resample_numeric_features(
        &numeric_features,
        5.0,
        va::transform::Aggregation::Mean,
    );
    assert_eq!(resampled.len(), 1);
    assert_eq!(resampled[0].name, "semantic.match_score");

    let mut dataset_with_features = dataset.clone();
    dataset_with_features.extend_records(
        feature_records
            .into_iter()
            .map(va::dataset_records::DatasetRecord::Feature),
    );
    assert!(dataset_with_features.features().count() > dataset.features().count());

    let mut scene_csv = Vec::new();
    va::output::write_scene_list_csv(&mut scene_csv, &[scene]).unwrap();
    let scene_csv = String::from_utf8(scene_csv).unwrap();
    assert!(scene_csv.contains("Scene Number,Start Frame"));
    assert!(scene_csv.contains("1,0,00:00:00.000"));
}

#[test]
fn subtitles_flow_through_linguistics_and_incremental_indexes() {
    let transcription = va::text_transcription::parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nRust cargo crates\n\n2\n00:00:01,000 --> 00:00:02,000\nBerlin roadmap launch\n\n3\n00:00:02,000 --> 00:00:03,000\nCargo build pipeline\n",
    )
    .unwrap();
    let linguistic = va::text_linguistics::analyze_transcription(
        &transcription,
        &va::text_linguistics::LinguisticAnalysisOptions::default(),
    )
    .unwrap();

    assert_eq!(linguistic.cues.len(), 3);
    assert!(linguistic
        .aggregate
        .tokens
        .iter()
        .any(|token| token.normalized == "cargo"));

    let segments = transcription
        .segments
        .iter()
        .map(va::text_transcription::segment_to_owned_text_segment)
        .collect::<Vec<_>>();

    let mut tfidf = va::text_corpus::TfIdfCorpus::default();
    let mut bm25 = va::text_corpus::Bm25Corpus::default();
    let embedder = va::text_semantics::HashedTextEmbedder::new(
        va::text_semantics::TextEmbeddingConfig {
            dimensions: 64,
            use_idf: true,
        },
        va::text_corpus::CorpusOptions::default(),
    )
    .unwrap();
    let mut semantic = va::text_semantics::SemanticTextIndex::new(embedder.clone());
    let mut embedding_index = va::text_semantics::EmbeddingSearchIndex::new(embedder);

    for segment in &segments {
        let segment = segment.as_segment();
        tfidf.add_text_segment("subs", &segment).unwrap();
        bm25.add_text_segment("subs", &segment).unwrap();
        semantic.add_text_segment("subs", &segment).unwrap();
        embedding_index.add_text_segment("subs", &segment).unwrap();
    }

    assert_eq!(tfidf.documents()[0].id, "subs:1");
    assert_eq!(bm25.documents()[2].id, "subs:3");
    assert_eq!(tfidf.search("cargo build", 1).unwrap()[0].id, "subs:3");
    assert_eq!(bm25.search("roadmap berlin", 1).unwrap()[0].id, "subs:2");
    assert_eq!(semantic.search("cargo crates", 1).unwrap()[0].id, "subs:1");
    assert_eq!(
        embedding_index.search("pipeline", 1).unwrap()[0].id,
        "subs:3"
    );
}
