use anyhow::Context;
use tokio::{
    fs,
    io::{AsyncRead, BufReader},
};

use std::{future::Future, path::Path};

/// A verifier of digital file signatures or hashes
pub trait AppVerifier: 'static + Clone {
    type Parameters;

    /// Verify `bin_path` using `parameters`, and return an error if this fails for any reason.
    fn verify(
        bin_path: impl AsRef<Path>,
        parameters: Self::Parameters,
    ) -> impl Future<Output = anyhow::Result<()>>;
}

/// Checksum verifier that uses SHA256
#[derive(Clone)]
pub struct Sha256Verifier;

impl AppVerifier for Sha256Verifier {
    /// The checksum
    type Parameters = [u8; 32];

    fn verify(
        bin_path: impl AsRef<Path>,
        expected_hash: Self::Parameters,
    ) -> impl Future<Output = anyhow::Result<()>> {
        let bin_path = bin_path.as_ref().to_owned();

        async move {
            let file = fs::File::open(&bin_path)
                .await
                .context(format!("Failed to open file at {}", bin_path.display()))?;
            let file = BufReader::new(file);

            Self::verify_inner(file, expected_hash).await
        }
    }
}

impl Sha256Verifier {
    async fn verify_inner(
        reader: impl AsyncRead + Unpin,
        expected_hash: [u8; 32],
    ) -> anyhow::Result<()> {
        let actual_hash = crate::hash::checksum(reader).await?;

        // Verify that hash is correct
        if expected_hash != actual_hash {
            anyhow::bail!("Invalid checksum for bin file");
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use rand::RngCore;
    use sha2::Digest;
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_sha256_checksum() {
        // Generate some random data
        let mut data = vec![0u8; 1024 * 1024];
        rand::thread_rng().fill_bytes(&mut data);

        // Hash it
        let mut hasher = sha2::Sha256::new();
        hasher.update(&data);
        let expected_hash = hasher.finalize();
        let expected_hash: [u8; 32] = expected_hash[..].try_into().unwrap();

        // Same data should be accepted
        Sha256Verifier::verify_inner(Cursor::new(&data), expected_hash)
            .await
            .expect("expected checksum match");

        // Compare the hash against some random data, which should fail
        rand::thread_rng().fill_bytes(&mut data);
        Sha256Verifier::verify_inner(Cursor::new(&data), expected_hash)
            .await
            .expect_err("expected checksum mismatch");
    }
}
