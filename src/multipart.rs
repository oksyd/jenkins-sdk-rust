//! Buffered multipart form data for file parameters, fingerprints, and plugin uploads.

use crate::{Error, Result};
use http::HeaderValue;
use std::sync::atomic::{AtomicU64, Ordering};

/// An ordered multipart form. Files are supplied as bytes; this type performs no I/O.
///
/// Uploads are buffered and preserve field order and binary bytes. Core file
/// parameter names must match the job's parameter definitions.
///
/// ```
/// use jenkins_sdk::{Multipart, api::jobs};
///
/// let form = Multipart::new()
///     .text("BRANCH", "main")?
///     .file("upload.bin", "input.bin", vec![0, 1, 2, 255])?;
/// let request = jobs::build_with_files("demo", form)?;
/// # Ok::<(), jenkins_sdk::Error>(())
/// ```
#[derive(Default)]
pub struct Multipart {
    parts: Vec<(String, Vec<u8>)>,
}

impl Multipart {
    /// Create an empty form.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a UTF-8 text field. Names must be nonempty without control characters.
    pub fn text(mut self, name: &str, value: impl Into<String>) -> Result<Self> {
        self.parts.push((
            format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n",
                quoted(name)?
            ),
            value.into().into_bytes(),
        ));
        Ok(self)
    }

    /// Append a binary file with media type `application/octet-stream`.
    /// Field names and filenames must be nonempty without control characters;
    /// quotes and backslashes are escaped.
    pub fn file(mut self, name: &str, filename: &str, bytes: impl Into<Vec<u8>>) -> Result<Self> {
        self.parts.push((
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n",
                quoted(name)?, quoted(filename)?
            ),
            bytes.into(),
        ));
        Ok(self)
    }

    pub(crate) fn encode(self) -> (HeaderValue, Vec<u8>) {
        static NEXT_BOUNDARY: AtomicU64 = AtomicU64::new(0);
        // A boundary must not occur in any part. Checking the actual payload
        // makes this guarantee independent of randomness and file contents.
        let boundary = loop {
            let candidate = format!(
                "jenkins-sdk-{:016x}",
                NEXT_BOUNDARY.fetch_add(1, Ordering::Relaxed)
            );
            if self.parts.iter().all(|(headers, body)| {
                !headers.contains(&candidate)
                    && !body
                        .windows(candidate.len())
                        .any(|window| window == candidate.as_bytes())
            }) {
                break candidate;
            }
        };
        let mut body = Vec::new();
        for (headers, bytes) in self.parts {
            body.extend_from_slice(format!("--{boundary}\r\n{headers}\r\n").as_bytes());
            body.extend(bytes);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        let content_type =
            HeaderValue::from_str(&format!("multipart/form-data; boundary={boundary}"))
                .expect("a generated ASCII boundary is a valid header value");
        (content_type, body)
    }
}

impl std::fmt::Debug for Multipart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Multipart")
            .field("parts", &self.parts.len())
            .finish()
    }
}

fn quoted(value: &str) -> Result<String> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(Error::InvalidInput {
            field: "multipart name",
            message: "must be nonempty and contain no control characters",
        });
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}
