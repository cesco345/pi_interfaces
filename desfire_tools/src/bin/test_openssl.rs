// src/bin/test_openssl.rs
use openssl::symm::{Cipher, Crypter, Mode};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Testing OpenSSL functionality...");
    
    // Create a simple test
    let key = [0x00; 8]; // 8-byte DES key of zeros
    let data = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]; // 8-byte data
    
    // Try to encrypt with DES
    let cipher = Cipher::des_cbc();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, &key, Some(&[0; 8]))?;
    crypter.pad(false);
    
    let mut output = vec![0; data.len() + cipher.block_size()];
    let count = crypter.update(&data, &mut output)?;
    let rest = crypter.finalize(&mut output[count..])?;
    output.truncate(count + rest);
    
    println!("Encrypted data: {:02X?}", output);
    println!("OpenSSL is working correctly!");
    
    Ok(())
}
