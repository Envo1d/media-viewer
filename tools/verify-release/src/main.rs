use ed25519_dalek::Verifier as _;
use ed25519_dalek::{Signature, VerifyingKey};
use std::{fs, path::Path, process};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: verify-release <exe-path> <sig-path> <public-key-hex>");
        eprintln!();
        eprintln!("  exe-path        Binary that was signed");
        eprintln!("  sig-path        64-byte raw Ed25519 signature file");
        eprintln!("  public-key-hex  32-byte verifying key as 64 hex chars");
        process::exit(1);
    }

    let exe_path = Path::new(&args[1]);
    let sig_path = Path::new(&args[2]);
    let key_hex = args[3].trim();

    if key_hex.len() != 64 {
        eprintln!(
            "ERROR: public key must be 64 hex characters (got {})",
            key_hex.len()
        );
        process::exit(1);
    }

    let key_bytes_vec = match hex::decode(key_hex) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: public key is not valid hex: {e}");
            process::exit(1);
        }
    };

    let key_array: [u8; 32] = match key_bytes_vec.try_into() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("ERROR: public key decoded to wrong length (expected 32 bytes)");
            process::exit(1);
        }
    };

    let verifying_key = match VerifyingKey::from_bytes(&key_array) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("ERROR: invalid Ed25519 public key: {e}");
            process::exit(1);
        }
    };

    let sig_bytes = match fs::read(sig_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: cannot read sig file `{}`: {e}", sig_path.display());
            process::exit(1);
        }
    };

    if sig_bytes.len() != 64 {
        eprintln!(
            "ERROR: signature file must be exactly 64 bytes (got {})",
            sig_bytes.len()
        );
        process::exit(1);
    }

    let sig_array: [u8; 64] = sig_bytes.try_into().unwrap();
    let signature = Signature::from_bytes(&sig_array);

    let exe_bytes = match fs::read(exe_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: cannot read exe `{}`: {e}", exe_path.display());
            process::exit(1);
        }
    };

    match verifying_key.verify(&exe_bytes, &signature) {
        Ok(()) => {
            println!("Signature verification: OK");
            println!("  File:   {}", exe_path.display());
            println!("  Sig:    {}", sig_path.display());
            println!("  Bytes:  {}", exe_bytes.len());
            println!("  PubKey: {key_hex}");
        }
        Err(e) => {
            eprintln!("VERIFICATION FAILED: {e}");
            eprintln!("  File:   {}", exe_path.display());
            eprintln!("  Sig:    {}", sig_path.display());
            eprintln!("  This binary may have been tampered with.");
            process::exit(1);
        }
    }
}
