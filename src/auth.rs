use base64::{Engine, engine::general_purpose::STANDARD};
use http::{HeaderValue, header::AUTHORIZATION};
use std::fmt;

/// Credentials shared by Jenkins requests and crumb fetching.
/// Debug output never includes the token.
#[derive(Clone)]
pub enum Auth {
    Basic { user: String, token: String },
    Bearer { token: String },
}

impl Auth {
    pub fn basic(user: impl Into<String>, token: impl Into<String>) -> Self {
        Self::Basic {
            user: user.into(),
            token: token.into(),
        }
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer {
            token: token.into(),
        }
    }

    pub(crate) fn secret(&self) -> &str {
        match self {
            Self::Basic { token, .. } | Self::Bearer { token } => token,
        }
    }

    pub(crate) fn header(&self) -> crate::Result<(http::HeaderName, HeaderValue)> {
        let value = match self {
            Self::Basic { user, token } => {
                format!("Basic {}", STANDARD.encode(format!("{user}:{token}")))
            }
            Self::Bearer { token } => format!("Bearer {token}"),
        };
        let mut value = HeaderValue::from_str(&value)?;
        value.set_sensitive(true);
        Ok((AUTHORIZATION, value))
    }
}

impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basic { user, .. } => f
                .debug_struct("Basic")
                .field("user", user)
                .finish_non_exhaustive(),
            Self::Bearer { .. } => f.write_str("Bearer { .. }"),
        }
    }
}
