use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fs;

fn main() {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    fs::write("nexa_signing.key", &private_hex).expect("write nexa_signing.key");
    fs::write("nexa_verify.pub", &public_hex).expect("write nexa_verify.pub");

    println!("=================================================================");
    println!(" Nexa Ed25519 Keypair Generated");
    println!("=================================================================");
    println!(" Private key -> nexa_signing.key  (KEEP SECRET -- add to .gitignore)");
    println!(" Public  key -> nexa_verify.pub   (safe to commit)");
    println!();
    println!(" Set in CI as NEXA_PUBLIC_KEY:");
    println!(" {public_hex}");
    println!();
    println!(" Rebuild the updater with:");
    println!(" NEXA_PUBLIC_KEY={public_hex} cargo build -p nexa-updater --release");
    println!("=================================================================");

    let gi = std::path::Path::new(".gitignore");
    let existing = if gi.exists() {
        fs::read_to_string(gi).unwrap_or_default()
    } else {
        String::new()
    };
    if !existing.contains("nexa_signing.key") {
        let mut updated = existing;
        updated.push_str("\n# Nexa signing key -- NEVER commit\nnexa_signing.key\n");
        let _ = fs::write(gi, updated);
        println!(" Added nexa_signing.key to .gitignore");
    }
}
