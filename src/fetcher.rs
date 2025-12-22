//! Documentation fetcher for downloading and extracting rustdoc from docs.rs.

use std::io::Cursor;
use std::path::PathBuf;

use async_zip::base::read::seek::ZipFileReader;
use futures_util::{AsyncReadExt as FuturesAsyncReadExt, TryStreamExt};
use tempfile::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::error::FetchError;

/// Client for fetching documentation from docs.rs.
pub struct DocsFetcher {
    client: reqwest::Client,
}

impl DocsFetcher {
    /// Creates a new documentation fetcher with appropriate settings.
    #[must_use]
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("rustxt/0.1.0 (https://github.com/leynos/rustxt)")
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Fetches and extracts crate documentation from docs.rs.
    ///
    /// # Arguments
    ///
    /// * `crate_name` - The name of the crate to fetch.
    /// * `version` - Optional specific version. If `None`, fetches the latest.
    ///
    /// # Returns
    ///
    /// A `TempDir` containing the extracted documentation. The directory will be
    /// automatically cleaned up when the `TempDir` is dropped.
    ///
    /// # Errors
    ///
    /// Returns `FetchError` if the download fails, the crate is not found, or
    /// extraction fails.
    pub async fn fetch(
        &self,
        crate_name: &str,
        version: Option<&str>,
    ) -> Result<(TempDir, String), FetchError> {
        let version_str = version.unwrap_or("latest");
        let url = format!("https://docs.rs/crate/{crate_name}/{version_str}/download");

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(FetchError::CrateNotFound {
                name: crate_name.to_owned(),
                version: version_str.to_owned(),
            });
        }

        let response = response.error_for_status()?;

        // Extract the actual version from the final URL (after redirects)
        let final_url = response.url().as_str();
        let actual_version = extract_version_from_url(final_url, crate_name)
            .unwrap_or_else(|| version_str.to_owned());

        // Download the ZIP file into memory
        let bytes = response
            .bytes_stream()
            .try_fold(Vec::new(), |mut acc, chunk| async move {
                acc.extend_from_slice(&chunk);
                Ok(acc)
            })
            .await?;

        // Create temp directory and extract
        let temp_dir = TempDir::new()?;
        extract_zip(&bytes, temp_dir.path()).await?;

        Ok((temp_dir, actual_version))
    }
}

impl Default for DocsFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the version from a docs.rs URL.
fn extract_version_from_url(url: &str, crate_name: &str) -> Option<String> {
    // URL format: https://docs.rs/crate/{name}/{version}/download
    let pattern = format!("/crate/{crate_name}/");
    let start = url.find(&pattern)? + pattern.len();
    let rest = url.get(start..)?;
    let end = rest.find('/')?;
    rest.get(..end).map(ToOwned::to_owned)
}

/// Extracts a ZIP archive to the specified directory.
async fn extract_zip(data: &[u8], target_dir: &std::path::Path) -> Result<(), FetchError> {
    let cursor = Cursor::new(data);
    let compat_reader = cursor.compat();
    let mut zip = ZipFileReader::new(compat_reader).await?;

    let entries: Vec<_> = zip
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            let filename = entry.filename().as_str().ok()?.to_owned();
            Some((i, filename))
        })
        .collect();

    for (index, filename) in entries {
        let target_path = target_dir.join(&filename);

        // Skip if path escapes target directory (security check)
        if !target_path.starts_with(target_dir) {
            continue;
        }

        if filename.ends_with('/') {
            // Directory entry
            fs::create_dir_all(&target_path).await?;
        } else {
            // File entry
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            let mut entry_reader = zip.reader_with_entry(index).await?;
            let mut file = fs::File::create(&target_path).await?;

            let mut buf = vec![0u8; 8192];
            loop {
                let n = FuturesAsyncReadExt::read(&mut entry_reader, &mut buf).await?;
                if n == 0 {
                    break;
                }
                file.write_all(buf.get(..n).unwrap_or(&[])).await?;
            }
        }
    }

    Ok(())
}

/// Finds the crate's documentation root directory within the extracted archive.
///
/// The ZIP archive may contain documentation at the root or in a subdirectory
/// named after the crate. This function locates the directory containing
/// `index.html` for the crate.
pub fn find_docs_root(temp_dir: &TempDir, crate_name: &str) -> Option<PathBuf> {
    let base = temp_dir.path();

    // Check for crate directory at root (e.g., /gpui/index.html)
    let crate_dir = base.join(crate_name);
    if crate_dir.join("index.html").exists() {
        return Some(crate_dir);
    }

    // Check for snake_case variant (e.g., crate-name -> crate_name)
    let snake_name = crate_name.replace('-', "_");
    let snake_dir = base.join(&snake_name);
    if snake_dir.join("index.html").exists() {
        return Some(snake_dir);
    }

    // Check if index.html is at root
    if base.join("index.html").exists() {
        return Some(base.to_path_buf());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_url() {
        let url = "https://docs.rs/crate/clap/4.5.0/download";
        assert_eq!(
            extract_version_from_url(url, "clap"),
            Some("4.5.0".to_owned())
        );

        let url = "https://docs.rs/crate/tokio/1.0.0/download";
        assert_eq!(
            extract_version_from_url(url, "tokio"),
            Some("1.0.0".to_owned())
        );
    }
}
