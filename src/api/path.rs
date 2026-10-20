//! Validated Jenkins names and nested resource paths.

use crate::Result;

pub(crate) fn validate_segment(segment: &str) -> Result<()> {
    if segment.is_empty()
        || matches!(segment, "." | "..")
        || segment.contains(['/', '\\'])
        || segment.chars().any(char::is_control)
    {
        return Err(crate::Error::InvalidInput {
            field: "path segment",
            message: "must be nonempty and contain no separators, dot segments, or control characters",
        });
    }
    Ok(())
}

pub(super) fn segments(path: &str) -> Result<Vec<String>> {
    path.split('/')
        .map(|part| {
            validate_segment(part)?;
            Ok(part.to_owned())
        })
        .collect()
}

pub(super) fn job_segments(job: &str, suffix: &[&str]) -> Result<Vec<String>> {
    let mut segments: Vec<_> = segments(job)?
        .into_iter()
        .flat_map(|part| ["job".to_owned(), part])
        .collect();
    segments.extend(suffix.iter().map(|part| (*part).to_owned()));
    Ok(segments)
}
