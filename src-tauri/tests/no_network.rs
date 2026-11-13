//! ADR 0016: the crate makes no network request, so no network crate may be declared or resolved.

use std::path::Path;
use std::process::Command;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

const BANNED_CRATES: [&str; 11] = [
    "reqwest",
    "hyper",
    "ureq",
    "isahc",
    "surf",
    "attohttpc",
    "curl",
    "tauri-plugin-updater",
    "tauri-plugin-http",
    "tauri-plugin-websocket",
    "tungstenite",
];

fn dependency_names(manifest: &str) -> Vec<String> {
    let mut in_dependencies = false;
    let mut names = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[') {
            let header = header.trim_end_matches(']');
            in_dependencies = header.ends_with("dependencies");
            if let Some((_, name)) = header.split_once("dependencies.") {
                names.push(name.to_string());
            }
            continue;
        }
        if in_dependencies
            && !line.starts_with('#')
            && let Some((name, _)) = line.split_once('=')
        {
            names.push(name.trim().trim_matches('"').to_string());
        }
    }
    names
}

#[test]
fn the_manifest_declares_no_network_dependency() {
    let manifest = std::fs::read_to_string(Path::new(MANIFEST_DIR).join("Cargo.toml")).unwrap();
    let names = dependency_names(&manifest);
    assert!(names.iter().any(|n| n == "tauri"), "{names:?}");
    for name in names {
        for banned in BANNED_CRATES {
            assert!(!name.contains(banned), "`{name}` is a network dependency");
        }
    }
}

#[test]
fn the_resolved_dependency_tree_has_no_network_crate() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "tree",
            "--prefix",
            "none",
            "-e",
            "normal",
            "--offline",
            "--manifest-path",
        ])
        .arg(Path::new(MANIFEST_DIR).join("Cargo.toml"))
        .output()
        .expect("run cargo tree");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).unwrap();
    let crates: Vec<&str> = tree
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert!(crates.contains(&"tauri"), "{crates:?}");
    for banned in BANNED_CRATES {
        assert!(
            !crates.contains(&banned),
            "`{banned}` is in the dependency tree"
        );
    }
}
