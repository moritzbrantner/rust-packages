use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn text_core_stays_independent_of_specialized_text_and_audio_crates() {
    let manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/text/text-core/Cargo.toml"),
    )
    .expect("read text-core manifest");

    for forbidden in [
        "text-transcripts",
        "text-retrieval",
        "text-linguistics",
        "audio-analysis",
        "video-analysis-models",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "text-core must not depend on specialized contract consumer `{forbidden}`"
        );
    }
}

#[test]
fn audio_asr_contract_uses_text_transcript_contracts() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/audio/audio-analysis-models/src/lib.rs"),
    )
    .expect("read audio-analysis-models source");

    assert!(source
        .contains("pub use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};"));
    assert!(source.contains("pub imported_segments: Vec<TranscriptSegmentContract>"));
    assert!(source.contains("pub transcript: TranscriptionContract"));
    assert!(source
        .contains("#[deprecated(note = \"use text_transcripts::TranscriptSegmentContract\")]"));
}

#[test]
fn transcript_dtos_are_owned_by_text_transcripts_with_explicit_audio_compatibility_shim() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates");
    let mut violations = Vec::new();
    collect_rust_sources(&root, &mut |path| {
        let source = fs::read_to_string(path).expect("read Rust source");
        for (line_index, line) in source.lines().enumerate() {
            if !line.contains("pub struct Transcript") {
                continue;
            }
            let path_text = path.to_string_lossy();
            let allowed = path_text.contains("crates/text/text-transcripts/")
                || (path_text.contains("crates/audio/audio-analysis-models/src/lib.rs")
                    && line.contains("TranscriptSegmentPrediction"))
                || line.contains("TranscriptHeuristicAnalyzer")
                || line.contains("TranscriptStatsExtractor");
            if !allowed {
                violations.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "new public Transcript* DTOs must live in text-transcripts or be allowlisted compatibility shims: {}",
        violations.join(", ")
    );
}

fn collect_rust_sources(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "target" || file_name == ".cargo-target" || file_name == "vendor" {
            continue;
        }
        if path.is_dir() {
            collect_rust_sources(&path, visit);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            visit(&path);
        }
    }
}
