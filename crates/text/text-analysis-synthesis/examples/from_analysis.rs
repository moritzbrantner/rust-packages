use text_analysis_linguistics::{analyze_text, LinguisticAnalysisOptions};
use text_analysis_synthesis::{synthesize_from_analysis, TextSynthesisOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let analysis = analyze_text(
        "Alice presented the roadmap in Berlin.",
        &LinguisticAnalysisOptions::default(),
    )?;
    let generated = synthesize_from_analysis(
        "doc-1",
        &analysis,
        TextSynthesisOptions {
            sentence_count: 1,
            ..TextSynthesisOptions::default()
        },
    )?;

    println!("{}", generated.value.text);
    Ok(())
}
