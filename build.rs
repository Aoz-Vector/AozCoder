use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=schemas/runtime_envelope_v1.json");

    if let Ok(hash) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        if !hash.is_empty() {
            println!("cargo:rustc-env=GIT_HASH={hash}");
        }
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let schema_path = Path::new("schemas/runtime_envelope_v1.json");

    if schema_path.exists() {
        let content = fs::read(schema_path).expect("failed to read schema");
        let digest = hex::encode(Sha256::digest(&content));

        let stub = format!(
            r#"pub const SCHEMA_HASH: &str = "{digest}";

#[cfg(test)]
mod schema_tests {{
    use super::*;
    #[test]
    fn schema_hash_non_empty() {{
        assert_eq!(SCHEMA_HASH.len(), 64);
    }}
}}
"#
        );

        let dest = Path::new(&out_dir).join("schema_validated.rs");
        fs::write(dest, stub).expect("failed to write schema stub");
    }
}
