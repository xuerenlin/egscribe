use crypto::{aes, blockmodes, buffer};
use crypto::buffer::{ReadBuffer, WriteBuffer};
use crypto::pbkdf2::pbkdf2;
use crypto::hmac::Hmac;
use crypto::sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};

const SALT: &[u8] = b"fixed_salt_for_demo"; // 实际应用中应使用随机盐
const ITERATIONS: u32 = 10; // PBKDF2 迭代次数

/// 加密 content 并返回 Base64 编码的密文
pub fn enc_content(content: &str, passwd: &str) -> Result<String, String> {
    // 1. 生成密钥和 IV
    let (key, iv) = derive_key_and_iv(passwd)?;

    // 2. 初始化加密器
    let mut encryptor = aes::cbc_encryptor(
        aes::KeySize::KeySize256,
        &key,
        &iv,
        blockmodes::PkcsPadding,
    );

    // 3. 加密数据
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

    // 4. 返回 Base64 编码的密文
    Ok(general_purpose::STANDARD.encode(&result))
}

/// 解密 Base64 编码的密文并返回明文
pub fn dec_content(content: &str, passwd: &str) -> Result<String, String> {
    // 1. Base64 解码
    let ciphertext = match general_purpose::STANDARD.decode(content) {
        Ok(data) => data,
        Err(e) => return Err(format!("Base64 decode failed: {}", e)),
    };

    // 2. 生成密钥和 IV
    let (key, iv) = derive_key_and_iv(passwd)?;

    // 3. 初始化解密器
    let mut decryptor = aes::cbc_decryptor(
        aes::KeySize::KeySize256,
        &key,
        &iv,
        blockmodes::PkcsPadding,
    );

    // 4. 解密数据
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

    // 5. 返回 UTF-8 字符串
    match String::from_utf8(result) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("UTF-8 conversion failed: {}", e)),
    }
}

/// 使用 PBKDF2 从密码派生密钥和 IV
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