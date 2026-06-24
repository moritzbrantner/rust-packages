//! Internal module support for package catalog.

const CONTRACTS_MARKDOWN: &str = include_str!("../../../../docs/API_CONTRACTS.md");

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for package info.
pub struct PackageInfo {
    /// Human-readable name for this value.
    pub name: String,
    /// The role value.
    pub role: String,
    /// The capabilities value.
    pub capabilities: Vec<PackageCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for package capability.
pub struct PackageCapability {
    /// The kind value.
    pub kind: PackageCapabilityKind,
    /// The entrypoint value.
    pub entrypoint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing package capability kind.
pub enum PackageCapabilityKind {
    /// The library variant.
    Library,
    /// The cli variant.
    Cli,
    /// The API variant.
    Api,
    /// The ui variant.
    Ui,
}

impl PackageCapabilityKind {
    /// Borrows this value as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::Cli => "cli",
            Self::Api => "api",
            Self::Ui => "ui",
        }
    }
}

/// Returns package catalog.
pub fn package_catalog() -> Vec<PackageInfo> {
    let mut packages: Vec<PackageInfo> = parse_contract_table(CONTRACTS_MARKDOWN)
        .into_iter()
        .map(|row| PackageInfo {
            capabilities: capabilities_for(&row.name),
            name: row.name,
            role: row.role,
        })
        .collect();

    if !packages
        .iter()
        .any(|pkg| pkg.name == "@moritzbrantner/video-analysis-web")
    {
        packages.push(PackageInfo {
            name: "@moritzbrantner/video-analysis-web".to_string(),
            role: "Prototype web app exposing package endpoints and package UI.".to_string(),
            capabilities: capabilities_for("@moritzbrantner/video-analysis-web"),
        });
    }

    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

/// Returns package by name.
pub fn package_by_name(name: &str) -> Option<PackageInfo> {
    package_catalog().into_iter().find(|pkg| pkg.name == name)
}

fn capabilities_for(name: &str) -> Vec<PackageCapability> {
    vec![
        PackageCapability {
            kind: PackageCapabilityKind::Library,
            entrypoint: library_entrypoint(name),
        },
        PackageCapability {
            kind: PackageCapabilityKind::Cli,
            entrypoint: cli_entrypoint(name),
        },
        PackageCapability {
            kind: PackageCapabilityKind::Api,
            entrypoint: api_entrypoint(name),
        },
        PackageCapability {
            kind: PackageCapabilityKind::Ui,
            entrypoint: ui_entrypoint(name),
        },
    ]
}

fn library_entrypoint(name: &str) -> String {
    match name {
        "@moritzbrantner/video-analysis-ui" => {
            "import from @moritzbrantner/video-analysis-ui".to_string()
        }
        "@moritzbrantner/video-analysis-web" => "prototypes/web/video-analysis-web".to_string(),
        "moenarch-video-analysis-cli" => "use video_analysis_cli::package_catalog".to_string(),
        rust_crate => format!("use {}", short_package_name(rust_crate).replace('-', "_")),
    }
}

fn cli_entrypoint(name: &str) -> String {
    if name.starts_with('@') {
        return "frontend package scripts".to_string();
    }
    let short_name = short_package_name(name);
    format!("{short_name}/cli (package {name}-cli)")
}

fn api_entrypoint(name: &str) -> String {
    if name.starts_with('@') {
        return format!("/api/packages?name={}", percent_encode(name));
    }
    let short_name = short_package_name(name);
    format!("{short_name}/api (package {name}-server)")
}

fn ui_entrypoint(name: &str) -> String {
    if name.starts_with('@') {
        return format!("Architecture page package detail for {name}");
    }
    let short_name = short_package_name(name);
    format!("{short_name}/app (package @moritzbrantner/{short_name}-app)")
}

fn short_package_name(name: &str) -> &str {
    name.strip_prefix("moenarch-")
        .or_else(|| name.strip_prefix("moritzbrantner-"))
        .unwrap_or(name)
}

fn active_package_name(name: &str) -> String {
    if name.starts_with('@') {
        return name.to_string();
    }
    name.strip_prefix("moritzbrantner-")
        .map(|short| format!("moenarch-{short}"))
        .unwrap_or_else(|| name.to_string())
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

#[derive(Debug, Clone)]
struct ContractRow {
    name: String,
    role: String,
}

fn parse_contract_table(markdown: &str) -> Vec<ContractRow> {
    let Some((_, after_heading)) = markdown.split_once("## Workspace Contract Map") else {
        return Vec::new();
    };

    let mut table_lines = Vec::new();
    let mut in_table = false;
    for line in after_heading.lines() {
        let trimmed = line.trim();
        if !in_table {
            if trimmed.starts_with("| Package |") {
                in_table = true;
                table_lines.push(trimmed);
            }
            continue;
        }
        if !trimmed.starts_with('|') {
            break;
        }
        table_lines.push(trimmed);
    }

    table_lines
        .into_iter()
        .skip(2)
        .filter_map(parse_contract_row)
        .collect()
}

fn parse_contract_row(line: &str) -> Option<ContractRow> {
    let cells: Vec<String> = line
        .trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(clean_cell)
        .collect();

    Some(ContractRow {
        name: active_package_name(cells.first()?),
        role: cells.get(1)?.to_string(),
    })
}

fn clean_cell(value: &str) -> String {
    value
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_gives_every_package_four_capabilities() {
        let catalog = package_catalog();
        assert!(catalog
            .iter()
            .any(|pkg| pkg.name == "moenarch-video-analysis-core"));
        assert!(catalog
            .iter()
            .any(|pkg| pkg.name == "moenarch-video-analysis-cli"));
        assert!(catalog
            .iter()
            .any(|pkg| pkg.name == "@moritzbrantner/video-analysis-web"));

        for package in catalog {
            let kinds: Vec<&str> = package
                .capabilities
                .iter()
                .map(|capability| capability.kind.as_str())
                .collect();
            assert_eq!(kinds, ["library", "cli", "api", "ui"]);
        }
    }

    #[test]
    fn cli_crate_has_a_library_entrypoint() {
        let package = package_by_name("moenarch-video-analysis-cli").unwrap();
        assert!(package.capabilities.iter().any(|capability| capability.kind
            == PackageCapabilityKind::Library
            && capability.entrypoint.contains("video_analysis_cli")));
    }
}
