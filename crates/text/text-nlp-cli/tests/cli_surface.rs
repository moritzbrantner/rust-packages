use std::process::Command;

#[test]
fn models_command_lists_core_presets() {
    let output = Command::new(env!("CARGO_BIN_EXE_text-nlp"))
        .arg("models")
        .output()
        .expect("run text-nlp models");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("bert-base-ner"));
    assert!(stdout.contains("all-mpnet-base-v2"));
}

#[test]
fn sentiment_command_emits_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_text-nlp"))
        .args(["sentiment", "--text", "excellent reliable work"])
        .output()
        .expect("run text-nlp sentiment");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"operation\":\"sentiment\""));
    assert!(stdout.contains("\"runtime\":\"lexical\""));
}
