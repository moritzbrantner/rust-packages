use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn library_manifests_do_not_embed_generic_runtime_surfaces() {
    for manifest in workspace_manifests(Path::new(env!("CARGO_MANIFEST_DIR"))) {
        let manifest_text = fs::read_to_string(&manifest).unwrap();
        assert!(
            !manifest_text.contains("package-surfaces"),
            "{} still points at shared package-surface sources",
            manifest.display()
        );
        assert!(
            !manifest_text.contains("# Shared package surface targets."),
            "{} still declares generic runtime surface binaries",
            manifest.display()
        );
    }
}

fn workspace_manifests(root: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    collect_manifests(root, &mut manifests);
    manifests
}

fn collect_manifests(dir: &Path, manifests: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git"
            || file_name == ".cargo-target"
            || file_name == "target"
            || file_name == "vendor"
            || file_name == "node_modules"
        {
            continue;
        }
        if path.is_dir() {
            collect_manifests(&path, manifests);
        } else if file_name == "Cargo.toml" {
            manifests.push(path);
        }
    }
}
