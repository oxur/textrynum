//! Authenticated user identity and extraction helpers.

/// An authenticated user identity, extracted from a validated token.
///
/// Stored in HTTP request extensions by the auth middleware and propagated
/// through rmcp into MCP tool handler context via `Extension<Parts>`.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The user's email address.
    pub email: String,
    /// The user's unique subject identifier (from the `sub` claim).
    pub subject: String,
}

/// Extract the `AuthenticatedUser` from HTTP request `Parts`, if present.
pub fn user_from_parts(parts: &http::request::Parts) -> Option<&AuthenticatedUser> {
    parts.extensions.get::<AuthenticatedUser>()
}

/// Extract the user's email from HTTP request `Parts`.
///
/// Returns `"anonymous"` if no authenticated user is present (dev mode).
pub fn email_from_parts(parts: &http::request::Parts) -> &str {
    parts
        .extensions
        .get::<AuthenticatedUser>()
        .map(|u| u.email.as_str())
        .unwrap_or("anonymous")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts_with_user() -> http::request::Parts {
        let (mut parts, _body) = http::Request::new(()).into_parts();
        parts.extensions.insert(AuthenticatedUser {
            email: "alice@banyan.com".to_string(),
            subject: "sub_123".to_string(),
        });
        parts
    }

    fn parts_without_user() -> http::request::Parts {
        let (parts, _body) = http::Request::new(()).into_parts();
        parts
    }

    #[test]
    fn test_user_from_parts_present() {
        let parts = parts_with_user();
        let user = user_from_parts(&parts).unwrap();
        assert_eq!(user.email, "alice@banyan.com");
        assert_eq!(user.subject, "sub_123");
    }

    #[test]
    fn test_user_from_parts_absent() {
        let parts = parts_without_user();
        assert!(user_from_parts(&parts).is_none());
    }

    #[test]
    fn test_email_from_parts_present() {
        let parts = parts_with_user();
        assert_eq!(email_from_parts(&parts), "alice@banyan.com");
    }

    #[test]
    fn test_email_from_parts_anonymous() {
        let parts = parts_without_user();
        assert_eq!(email_from_parts(&parts), "anonymous");
    }

    #[test]
    fn test_authenticated_user_clone() {
        let user = AuthenticatedUser {
            email: "bob@banyan.com".to_string(),
            subject: "sub_456".to_string(),
        };
        let cloned = user.clone();
        assert_eq!(cloned.email, "bob@banyan.com");
        assert_eq!(cloned.subject, "sub_456");
    }
}
