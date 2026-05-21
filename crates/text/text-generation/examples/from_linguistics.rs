use text_generation::{MarkovChain, MarkovInputMode};
use text_linguistics::{analyze_text, LinguisticAnalysisOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let analysis = analyze_text(
        "Scene transitions follow strong visual changes.",
        &LinguisticAnalysisOptions::default(),
    )?;

    let mut chain = MarkovChain::new(2)?;
    chain.train_analysis(&analysis, MarkovInputMode::Lemma);

    for prediction in chain.predict_next(["scene", "transition"], 3)? {
        println!("{} {:.3}", prediction.token, prediction.probability);
    }

    Ok(())
}
