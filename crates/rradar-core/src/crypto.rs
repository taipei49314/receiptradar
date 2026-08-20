//! Cryptography: DEK, HKDF blob keys, Argon2id, XChaCha20-Poly1305.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use rand::Rng;
use sha2::Sha256;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const BACKUP_MAGIC: &[u8; 8] = b"RRBACKUP";
pub const BACKUP_VERSION: u16 = 1;
pub const SEALED_MAGIC: &[u8; 8] = b"RRSEALED";
pub const SEALED_VERSION: u16 = 1;

/// Argon2id params (design: m=64MiB, t=3, p=1) — reduced m for CI speed is OK via test helper.
pub const ARGON2_M_KIB: u32 = 64 * 1024;
pub const ARGON2_T: u32 = 3;
pub const ARGON2_P: u32 = 1;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("aead encrypt/decrypt failed")]
    Aead,
    #[error("argon2 failed: {0}")]
    Argon2(String),
    #[error("invalid magic or version")]
    BadHeader,
    #[error("ciphertext truncated")]
    Truncated,
    #[error("hkdf expand failed")]
    Hkdf,
}

/// 32-byte data encryption key; zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Dek([u8; 32]);

impl Dek {
    pub fn generate() -> Self {
        let mut k = [0u8; 32];
        rand::rng().fill_bytes(&mut k);
        Self(k)
    }

    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    rand::rng().fill_bytes(&mut b);
    b
}

fn argon2_params(m_kib: u32) -> Result<Params, CryptoError> {
    Params::new(m_kib, ARGON2_T, ARGON2_P, Some(32)).map_err(|e| CryptoError::Argon2(e.to_string()))
}

/// Passphrase + salt → 32-byte key (Argon2id).
pub fn derive_key_argon2id(
    passphrase: &[u8],
    salt: &[u8],
    m_kib: u32,
) -> Result<[u8; 32], CryptoError> {
    let params = argon2_params(m_kib)?;
    let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    a2.hash_password_into(passphrase, salt, &mut out)
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;
    Ok(out)
}

/// HKDF-SHA256 blob key: IKM=DEK, salt=zeros, info=`rradar-blob-v1`||receipt_id
pub fn blob_key(dek: &Dek, receipt_id: &str) -> Result<[u8; 32], CryptoError> {
    let hk = Hkdf::<Sha256>::new(Some(&[0u8; 32]), dek.as_bytes());
    let mut info = b"rradar-blob-v1".to_vec();
    info.extend_from_slice(receipt_id.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(&info, &mut okm).map_err(|_| CryptoError::Hkdf)?;
    Ok(okm)
}

pub fn aead_encrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let n: &XNonce = nonce.into();
    cipher
        .encrypt(
            n,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::Aead)
}

pub fn aead_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let n: &XNonce = nonce.into();
    cipher
        .decrypt(
            n,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CryptoError::Aead)
}

/// Seal arbitrary bytes (P2 at-rest: whole SQLite file).
/// Layout: magic(8) | version u16le | salt(16) | nonce(24) | ciphertext+tag
pub fn seal_bytes(passphrase: &str, plaintext: &[u8], m_kib: u32) -> Result<Vec<u8>, CryptoError> {
    let salt = random_bytes::<16>();
    let nonce = random_bytes::<24>();
    let key = derive_key_argon2id(passphrase.as_bytes(), &salt, m_kib)?;
    let ct = aead_encrypt(&key, &nonce, plaintext, SEALED_MAGIC)?;
    let mut out = Vec::with_capacity(8 + 2 + 16 + 24 + ct.len());
    out.extend_from_slice(SEALED_MAGIC);
    out.extend_from_slice(&SEALED_VERSION.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn unseal_bytes(passphrase: &str, sealed: &[u8], m_kib: u32) -> Result<Vec<u8>, CryptoError> {
    if sealed.len() < 8 + 2 + 16 + 24 + 16 {
        return Err(CryptoError::Truncated);
    }
    if &sealed[0..8] != SEALED_MAGIC {
        return Err(CryptoError::BadHeader);
    }
    let ver = u16::from_le_bytes([sealed[8], sealed[9]]);
    if ver != SEALED_VERSION {
        return Err(CryptoError::BadHeader);
    }
    let salt: [u8; 16] = sealed[10..26].try_into().unwrap();
    let nonce: [u8; 24] = sealed[26..50].try_into().unwrap();
    let ct = &sealed[50..];
    let key = derive_key_argon2id(passphrase.as_bytes(), &salt, m_kib)?;
    aead_decrypt(&key, &nonce, ct, SEALED_MAGIC)
}

/// backup.rradar v1 (same crypto as sealed; AAD = magic).
pub fn seal_backup(passphrase: &str, plaintext: &[u8], m_kib: u32) -> Result<Vec<u8>, CryptoError> {
    let salt = random_bytes::<16>();
    let nonce = random_bytes::<24>();
    let key = derive_key_argon2id(passphrase.as_bytes(), &salt, m_kib)?;
    let ct = aead_encrypt(&key, &nonce, plaintext, BACKUP_MAGIC)?;
    let mut out = Vec::with_capacity(8 + 2 + 4 + 16 + 24 + ct.len());
    out.extend_from_slice(BACKUP_MAGIC);
    out.extend_from_slice(&BACKUP_VERSION.to_le_bytes());
    out.extend_from_slice(&m_kib.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn unseal_backup(passphrase: &str, sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if sealed.len() < 8 + 2 + 4 + 16 + 24 + 16 {
        return Err(CryptoError::Truncated);
    }
    if &sealed[0..8] != BACKUP_MAGIC {
        return Err(CryptoError::BadHeader);
    }
    let ver = u16::from_le_bytes([sealed[8], sealed[9]]);
    if ver != BACKUP_VERSION {
        return Err(CryptoError::BadHeader);
    }
    let m_kib = u32::from_le_bytes([sealed[10], sealed[11], sealed[12], sealed[13]]);
    let salt: [u8; 16] = sealed[14..30].try_into().unwrap();
    let nonce: [u8; 24] = sealed[30..54].try_into().unwrap();
    let ct = &sealed[54..];
    let key = derive_key_argon2id(passphrase.as_bytes(), &salt, m_kib)?;
    aead_decrypt(&key, &nonce, ct, BACKUP_MAGIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST_M: u32 = 8; // KiB — tests only

    #[test]
    fn seal_roundtrip() {
        let pt = b"hello ledger bytes";
        let sealed = seal_bytes("correct horse", pt, FAST_M).unwrap();
        let out = unseal_bytes("correct horse", &sealed, FAST_M).unwrap();
        assert_eq!(out, pt);
        assert!(unseal_bytes("wrong", &sealed, FAST_M).is_err());
    }

    #[test]
    fn backup_roundtrip() {
        let pt = b"tar-or-json-payload";
        let sealed = seal_backup("pw", pt, FAST_M).unwrap();
        assert_eq!(&sealed[0..8], BACKUP_MAGIC);
        let out = unseal_backup("pw", &sealed).unwrap();
        assert_eq!(out, pt);
    }

    #[test]
    fn blob_key_stable() {
        let dek = Dek::from_bytes([7u8; 32]);
        let a = blob_key(&dek, "id1").unwrap();
        let b = blob_key(&dek, "id1").unwrap();
        let c = blob_key(&dek, "id2").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn legacy_crypto_vectors_are_stable() {
        let dek = Dek::from_bytes([7u8; 32]);
        assert_eq!(
            hex::encode(blob_key(&dek, "id1").unwrap()),
            "5b8575d6de47d3fe5a1c41c6017e6d490685db007be97c923f213232c7bbbe10"
        );

        let ciphertext =
            aead_encrypt(&[0x11; 32], &[0x22; 24], b"receipt-v1", BACKUP_MAGIC).unwrap();
        assert_eq!(
            hex::encode(&ciphertext),
            "f102846935c990b9937cb728fd67058195b70e83c6d74359ad23"
        );
        assert_eq!(
            aead_decrypt(&[0x11; 32], &[0x22; 24], &ciphertext, BACKUP_MAGIC).unwrap(),
            b"receipt-v1"
        );
    }
}
