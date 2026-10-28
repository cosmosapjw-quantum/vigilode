use sha2::{Digest, Sha256};

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn sha256_f64_slice_bytes(values: &[f64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for v in values {
        hasher.update(v.to_bits().to_le_bytes());
    }
    hasher.finalize().into()
}
