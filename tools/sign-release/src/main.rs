use ed25519_dalek::{Signer, SigningKey};
use std::{fs, path::PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: sign-release <exe-path> <signing-key-file>");
        eprintln!("  signing-key-file: path to nexa_signing.key (64-char hex)");
        std::process::exit(1);
    }

    let exe_path = PathBuf::from(&args[1]);
    let key_path = PathBuf::from(&args[2]);

    let key_hex = fs::read_to_string(&key_path).unwrap_or_else(|e| {
        eprintln!("Cannot read key file: {e}");
        std::process::exit(1);
    });
    let key_hex = key_hex.trim();

    let key_bytes_vec = hex::decode(key_hex).unwrap_or_else(|e| {
        eprintln!("Key file is not valid hex: {e}");
        std::process::exit(1);
    });

    if key_bytes_vec.len() != 32 {
        eprintln!(
            "Key must be 32 bytes (64 hex chars), got {}",
            key_bytes_vec.len()
        );
        std::process::exit(1);
    }

    let key_array: [u8; 32] = key_bytes_vec.try_into().unwrap();
    let signing_key = SigningKey::from_bytes(&key_array);

    let exe_bytes = fs::read(&exe_path).unwrap_or_else(|e| {
        eprintln!("Cannot read exe: {e}");
        std::process::exit(1);
    });

    let signature = signing_key.sign(&exe_bytes);
    let sig_bytes = signature.to_bytes();

    let sig_path = {
        let mut p = exe_path.clone();
        let mut name = p.file_name().unwrap().to_os_string();
        name.push(".sig");
        p.set_file_name(name);
        p
    };

    fs::write(&sig_path, &sig_bytes).unwrap_or_else(|e| {
        eprintln!("Cannot write sig: {e}");
        std::process::exit(1);
    });

    let verifying_key = signing_key.verifying_key();

    use ed25519_dalek::Verifier as _;
    verifying_key
        .verify(&exe_bytes, &signature)
        .unwrap_or_else(|e| {
            eprintln!("Self-verification FAILED (bug): {e}");
            let _ = fs::remove_file(&sig_path);
            std::process::exit(1);
        });

    println!("Signed:  {}", exe_path.display());
    println!("Output:  {}", sig_path.display());
    println!(
        "Size:    {} bytes (exe)  +  64 bytes (sig)",
        exe_bytes.len()
    );
    println!("PubKey:  {}", hex::encode(verifying_key.to_bytes()));
    println!("Self-verification: OK");
}
