use sha2::{Digest, Sha256};

pub fn hash_bytes(content: &[u8]) -> String {
    hex::encode(Sha256::digest(content))
}
