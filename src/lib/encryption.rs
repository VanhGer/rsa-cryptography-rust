use crate::key::{Key, KeyVariant};
use indicatif::ProgressStyle;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

trait SizeInBytes {
    fn size_in_bytes(&self) -> usize;
}

impl SizeInBytes for BigUint {
    /// Returns the number of bytes needed to store all the bits of N-1
    fn size_in_bytes(&self) -> usize {
        (self.bits() / 8).to_usize().unwrap_or(0usize)
    }
}

impl Key {
    const ENCRYPTION_BYTE_OFFSET: usize = 1;
    const DEFAULT_ENCRYPTED_FILE_EXTENSION: &str = "cypher";
    const DEFAULT_ENCRYPTED_FILE_NAME: &str = "encrypted";
    const DEFAULT_DECRYPTED_FILE_EXTENSION: &str = "message";
    const DEFAULT_DECRYPTED_FILE_NAME: &str = "decrypted";

    fn open_input_output(&self, file_path: PathBuf, out_path: Option<PathBuf>) -> (File, File) {
        let file_path = {
            if file_path.is_file() {
                file_path
            } else {
                panic!("Path '{}' is inexistent", file_path.to_string_lossy());
            }
        };
        let out_path = {
            if let Some(out_path) = out_path {
                if out_path.is_file() {
                    match self.variant {
                        crate::key::KeyVariant::PublicKey => {
                            out_path.with_extension(Key::DEFAULT_ENCRYPTED_FILE_EXTENSION)
                        }
                        crate::key::KeyVariant::PrivateKey => {
                            out_path.with_extension(Key::DEFAULT_DECRYPTED_FILE_EXTENSION)
                        }
                    }
                } else if out_path.is_dir() {
                    match self.variant {
                        crate::key::KeyVariant::PublicKey => out_path
                            .join(Key::DEFAULT_ENCRYPTED_FILE_NAME)
                            .with_extension(Key::DEFAULT_ENCRYPTED_FILE_EXTENSION),
                        crate::key::KeyVariant::PrivateKey => out_path
                            .join(Key::DEFAULT_DECRYPTED_FILE_NAME)
                            .with_extension(Key::DEFAULT_DECRYPTED_FILE_EXTENSION),
                    }
                } else {
                    create_dir_all(out_path.parent().unwrap_or(Path::new(".")))
                        .expect("Failed to create parents directories");
                    out_path
                }
            } else {
                match self.variant {
                    crate::key::KeyVariant::PublicKey => PathBuf::from(".")
                        .join(Key::DEFAULT_ENCRYPTED_FILE_NAME)
                        .with_extension(Key::DEFAULT_ENCRYPTED_FILE_EXTENSION),
                    crate::key::KeyVariant::PrivateKey => PathBuf::from(".")
                        .join(Key::DEFAULT_DECRYPTED_FILE_NAME)
                        .with_extension(Key::DEFAULT_DECRYPTED_FILE_EXTENSION),
                }
            }
        };

        println!("Reading input file from `{}`", file_path.to_string_lossy());
        println!("Writting output file to `{}`", out_path.to_string_lossy());

        let file_in = File::open(file_path).expect("Error opening input file");
        let file_out = File::create(out_path).expect("Error opening output file");

        (file_in, file_out)
    }

    /// Encrypts a file chunk by chunk
    pub fn encrypt_file(&self, file_path: PathBuf, out_path: Option<PathBuf>) {
        assert_eq!(self.variant, KeyVariant::PublicKey);
        let (mut file_in, mut file_out) = self.open_input_output(file_path, out_path);
        let max_bytes_read = self.modulus.size_in_bytes() - Key::ENCRYPTION_BYTE_OFFSET; // always > 0 because min key size is 32 bits == 4 bytes
        let max_bytes_write = self.modulus.size_in_bytes() + Key::ENCRYPTION_BYTE_OFFSET;
        let mut source_bytes = vec![0u8; max_bytes_read];
        let mut destiny_bytes = Vec::<u8>::with_capacity(max_bytes_read);
        let mut bytes_amount_read = max_bytes_read;
        let pb = indicatif::ProgressBar::new(file_in.metadata().unwrap().len())
            .with_style(ProgressStyle::with_template("{msg} {bytes}/{total_bytes}").unwrap())
            .with_message("Encrypting");

        while bytes_amount_read == max_bytes_read {
            source_bytes.fill(0u8);
            bytes_amount_read = file_in.read(&mut source_bytes).unwrap();
            if bytes_amount_read == 0 {
                break;
            }
            let message = BigUint::from_bytes_le(&source_bytes);
            let encrypted = message.modpow(&self.exponent, &self.modulus);
            destiny_bytes.clear();
            let _ = destiny_bytes.write(&encrypted.to_bytes_le()).unwrap();
            let size_diff = (max_bytes_write) - destiny_bytes.len();
            destiny_bytes.append(&mut vec![0u8; size_diff]);
            let _bytes_amount_written = file_out.write(&destiny_bytes).unwrap();
            pb.inc(bytes_amount_read as u64);
        }
        pb.finish_with_message("Successfully encrypted");
    }

    /// decrypts a file chunk by chunk
    pub fn decrypt_file(&self, file_path: PathBuf, out_path: Option<PathBuf>) {
        assert_eq!(self.variant, KeyVariant::PrivateKey);
        let (mut file_in, mut file_out) = self.open_input_output(file_path, out_path);
        let max_bytes = self.modulus.size_in_bytes() + Key::ENCRYPTION_BYTE_OFFSET;
        let mut source_bytes = vec![0u8; max_bytes];
        let mut destiny_bytes = Vec::<u8>::with_capacity(max_bytes);
        let mut bytes_amount_read = max_bytes;

        let pb = indicatif::ProgressBar::new(file_in.metadata().unwrap().len())
            .with_style(ProgressStyle::with_template("{msg} {bytes}/{total_bytes}").unwrap())
            .with_message("Decrypting");

        while bytes_amount_read == max_bytes {
            source_bytes.fill(0u8);
            bytes_amount_read = file_in.read(&mut source_bytes).unwrap();
            if bytes_amount_read == 0 {
                break;
            }
            let encrypted = BigUint::from_bytes_le(&source_bytes);
            let message = encrypted.modpow(&self.exponent, &self.modulus);
            destiny_bytes.clear();
            let _ = destiny_bytes.write(&message.to_bytes_le()).unwrap();
            let _bytes_amount_written = file_out.write(&destiny_bytes).unwrap();
            pb.inc(bytes_amount_read as u64);
        }
        pb.finish_with_message("Successfully decrypted");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::KeyPair;
    use lipsum::lipsum;

    #[test]
    fn test_encrypt_decrypt() {
        let plain_file = Path::new("messages/lorem.txt");
        create_dir_all("./messages").unwrap();
        let mut test_file = File::create(plain_file).unwrap();
        let _res = test_file.write(lipsum(200).as_bytes()).unwrap();

        let public_key = Key {
            exponent: BigUint::from(65_537u32), // default value isn't present in key file
            modulus: BigUint::from(2523461377u64), // 0x9668f701
            variant: KeyVariant::PublicKey,
        };
        let private_key = Key {
            exponent: BigUint::from(343637873u32), // 0x147b7f71
            modulus: BigUint::from(2523461377u64), // 0x9668f701
            variant: KeyVariant::PrivateKey,
        };
        let keypair = KeyPair {
            public_key,
            private_key,
        };
        let file_path = PathBuf::from("./keys/tests/key_pair");
        keypair.write_keypair_files(Some(file_path)).unwrap();

        let encrypted = Some(PathBuf::from("messages/"));
        let decrypted = Some(PathBuf::from("messages/"));

        let pub_path = Some(PathBuf::from("keys/tests/key_pair.pub"));
        let priv_path = Some(PathBuf::from("keys/tests/key_pair"));
        let pub_key = Key::read_key_file(pub_path, crate::key::KeyVariant::PublicKey).unwrap();
        let priv_key = Key::read_key_file(priv_path, crate::key::KeyVariant::PrivateKey).unwrap();
        pub_key.encrypt_file(plain_file.to_path_buf(), encrypted);
        let encrypted = Some(PathBuf::from("messages/encrypted.cypher"));
        priv_key.decrypt_file(encrypted.unwrap(), decrypted);
    }
}
