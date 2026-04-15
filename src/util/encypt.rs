use crypto::{aes, blockmodes, buffer};
use crypto::buffer::{ReadBuffer, WriteBuffer};
use crypto::pbkdf2::pbkdf2;
use crypto::hmac::Hmac;
use crypto::sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};

const SALT: &[u8] = b"fixed_salt_for_demo"; // In actual applications, random salt should be used
const ITERATIONS: u32 = 10; // PBKDF2 iteration count

/// Encrypt content and return Base64 encoded ciphertext
pub fn enc_content(content: &str, passwd: &str) -> Result<String, String> {
    // 1. Generate key and IV
    let (key, iv) = derive_key_and_iv(passwd)?;

    // 2. Initialize encryptor
    let mut encryptor = aes::cbc_encryptor(
        aes::KeySize::KeySize256,
        &key,
        &iv,
        blockmodes::PkcsPadding,
    );

    // 3. Encrypt data
    let mut result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(content.as_bytes());
    let mut buffer = [0; 4096];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result_code = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true);
        result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());

        match result_code {
            Ok(_) => break,
            Err(crypto::symmetriccipher::SymmetricCipherError::InvalidLength) => {
                return Err("Invalid input length".to_string());
            }
            Err(e) => return Err(format!("Encryption failed: {:?}", e)),
        }
    }

    // 4. Return Base64 encoded ciphertext
    Ok(general_purpose::STANDARD.encode(&result))
}

/// Decrypt Base64 encoded ciphertext and return plaintext
pub fn dec_content(content: &str, passwd: &str) -> Result<String, String> {
    // 1. Base64 decode
    let ciphertext = match general_purpose::STANDARD.decode(content) {
        Ok(data) => data,
        Err(e) => return Err(format!("Base64 decode failed: {}", e)),
    };

    // 2. Generate key and IV
    let (key, iv) = derive_key_and_iv(passwd)?;

    // 3. Initialize decryptor
    let mut decryptor = aes::cbc_decryptor(
        aes::KeySize::KeySize256,
        &key,
        &iv,
        blockmodes::PkcsPadding,
    );

    // 4. Decrypt data
    let mut result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(&ciphertext);
    let mut buffer = [0; 4096];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result_code = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true);
        result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());

        match result_code {
            Ok(_) => break,
            Err(crypto::symmetriccipher::SymmetricCipherError::InvalidLength) => {
                return Err("Invalid ciphertext length".to_string());
            }
            Err(e) => return Err(format!("Decryption failed: {:?}", e)),
        }
    }

    // 5. Return UTF-8 string
    match String::from_utf8(result) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("UTF-8 conversion failed: {}", e)),
    }
}

/// Derive key and IV from password using PBKDF2
fn derive_key_and_iv(passwd: &str) -> Result<([u8; 32], [u8; 16]), String> {
    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    let mut output = [0u8; 48]; // 32 (key) + 16 (iv)

    let mut hmac = Hmac::new(Sha256::new(), passwd.as_bytes());
    pbkdf2(&mut hmac, SALT, ITERATIONS, &mut output);

    key.copy_from_slice(&output[..32]);
    iv.copy_from_slice(&output[32..48]);

    Ok((key, iv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enc_dec() {
        let content = "Hello, AES-256!";
        let passwd = "my_secure_password";

        let encrypted = enc_content(content, passwd).unwrap();
        let decrypted = dec_content(&encrypted, passwd).unwrap();

        println!("encrypted={}", encrypted);
        assert_eq!(content, decrypted);
    }
}