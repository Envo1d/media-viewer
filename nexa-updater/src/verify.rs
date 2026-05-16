use ed25519_dalek::Verifier as _;
use ed25519_dalek::{Signature, VerifyingKey};
use std::fs;
use std::path::Path;

const PUBLIC_KEY_HEX: &str = env!("NEXA_PUBLIC_KEY");

#[derive(Debug)]
pub enum VerifyError {
    InvalidPublicKey(String),

    InvalidSignatureLength { got: usize },

    SignatureMismatch,

    Io(std::io::Error),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPublicKey(e) => write!(f, "embedded public key is invalid: {e}"),
            Self::InvalidSignatureLength { got } => {
                write!(f, "signature file must be 64 bytes, got {got}")
            }
            Self::SignatureMismatch => {
                write!(f, "signature verification FAILED — binary may be tampered")
            }
            Self::Io(e) => write!(f, "I/O error during verification: {e}"),
        }
    }
}

impl From<std::io::Error> for VerifyError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub fn verify_file(exe_path: &Path, sig_path: &Path) -> Result<(), VerifyError> {
    let key_bytes_vec =
        hex::decode(PUBLIC_KEY_HEX).map_err(|e| VerifyError::InvalidPublicKey(e.to_string()))?;

    let key_bytes: [u8; 32] = key_bytes_vec
        .try_into()
        .map_err(|_| VerifyError::InvalidPublicKey("not 32 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| VerifyError::InvalidPublicKey(e.to_string()))?;

    let sig_bytes = fs::read(sig_path)?;
    if sig_bytes.len() != 64 {
        return Err(VerifyError::InvalidSignatureLength {
            got: sig_bytes.len(),
        });
    }
    let sig_array: [u8; 64] = sig_bytes.try_into().unwrap();
    let signature = Signature::from_bytes(&sig_array);

    let exe_bytes = fs::read(exe_path)?;

    verifying_key
        .verify(&exe_bytes, &signature)
        .map_err(|_| VerifyError::SignatureMismatch)?;

    tracing::info!(
        exe  = %exe_path.display(),
        "Ed25519 signature verified OK"
    );
    Ok(())
}
