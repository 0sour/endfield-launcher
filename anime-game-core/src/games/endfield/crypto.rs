use std::path::Path;

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

/// AES-256-CBC key retrieved via reverse engineering of the official launcher
const AES_KEY: [u8; 32] = [
    0xC0, 0xF3, 0x0E, 0x1C, 0xE7, 0x63, 0xBB, 0xC2, 0x1C, 0xC3, 0x55, 0xA3, 0x43, 0x03, 0xAC, 0x50,
    0x39, 0x94, 0x44, 0xBF, 0xF6, 0x8C, 0x4A, 0x22, 0xAF, 0x39, 0x8C, 0x0A, 0x16, 0x6E, 0xE1, 0x43
];

/// AES-256-CBC IV retrieved via reverse engineering of the official launcher
const AES_IV: [u8; 16] = [
    0x33, 0x46, 0x78, 0x61, 0x19, 0x27, 0x50, 0x64, 0x95, 0x01, 0x93, 0x72, 0x64, 0x60, 0x84, 0x00
];

/// Decrypt raw bytes to a UTF-8 string.
///
/// Used for in-memory decryption of CDN manifest files (game_files).
pub fn decrypt_bytes_to_string(encrypted: &[u8]) -> anyhow::Result<String> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }

    let mut buf = encrypted.to_vec();

    let plaintext = Aes256CbcDec::new(&AES_KEY.into(), &AES_IV.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| anyhow::anyhow!("Failed to decrypt data"))?;

    Ok(String::from_utf8_lossy(plaintext).to_string())
}

/// Decrypt a file and return its content as a UTF-8 string.
///
/// Suitable for text-based files like `config.ini` or JSON-formatted
/// verification lists.
pub fn decrypt_file_to_string(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;

    decrypt_bytes_to_string(&bytes)
}

/// Encrypt a UTF-8 string to raw bytes.
///
/// Used for writing back modified `config.ini` files.
pub fn encrypt_string_to_bytes(content: impl AsRef<str>) -> anyhow::Result<Vec<u8>> {
    let plaintext = content.as_ref().as_bytes();

    // Encrypt with PKCS7 padding (allocating convenience method)
    let ciphertext = Aes256CbcEnc::new(&AES_KEY.into(), &AES_IV.into())
        .encrypt_padded_vec_mut::<Pkcs7>(plaintext);

    Ok(ciphertext)
}

/// Encrypt a UTF-8 string and write it to a file.
///
/// Used for saving modified `config.ini` back to the game folder.
pub fn encrypt_string_to_file(content: impl AsRef<str>, path: &Path) -> anyhow::Result<()> {
    let encrypted = encrypt_string_to_bytes(content)?;

    std::fs::write(path, encrypted)?;

    Ok(())
}

/// Read the game version from an encrypted `config.ini` file
///
/// Returns `None` if the file doesn't exist or the version can't be found.
pub fn read_version_from_config(path: &Path) -> anyhow::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = decrypt_file_to_string(path)?;

    for line in content.lines() {
        let line = line.trim();

        if let Some(version) = line.strip_prefix("version=") {
            return Ok(Some(version.trim().to_string()));
        }
    }

    Ok(None)
}

/// Check if the given `config.ini` file is valid (decryptable and contains
/// a version line)
pub fn is_valid_config(path: &Path) -> bool {
    read_version_from_config(path)
        .map(|version| version.is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_known_data() {
        // 用 openssl 加密一段已知数据，验证解密
        // 这里直接测试空输入和错误输入的处理
        assert_eq!(decrypt_bytes_to_string(&[]).unwrap(), "");
        assert!(decrypt_bytes_to_string(&[0x01, 0x02, 0x03]).is_err());
    }
}
