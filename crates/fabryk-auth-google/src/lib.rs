//! Google OAuth2 token validation for Fabryk.
//!
//! Implements [`fabryk_auth::TokenValidator`] for Google:
//! - JWT id_token validation via JWKS
//! - Opaque access_token validation via Google userinfo endpoint
//! - JWKS key caching with TTL-based refresh

use std::future::Future;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use fabryk_auth::{AuthConfig, AuthError, AuthenticatedUser, TokenValidator};

/// TTL for cached JWKS keys (1 hour).
const JWKS_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Google's userinfo endpoint — validates opaque access tokens.
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// A single JSON Web Key from Google's JWKS endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct Jwk {
    /// Key ID — matched against the JWT header's `kid`.
    pub kid: String,
    /// RSA modulus (base64url-encoded).
    pub n: String,
    /// RSA exponent (base64url-encoded).
    pub e: String,
}

/// The JWKS response from Google's endpoint.
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

/// Cached JWKS keys with fetch timestamp.
struct CachedKeys {
    keys: Vec<Jwk>,
    fetched_at: Instant,
}

/// Google ID token claims.
#[derive(Debug, Deserialize)]
struct GoogleClaims {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    hd: Option<String>,
}

/// Response from Google's userinfo endpoint.
#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    hd: Option<String>,
}

/// Google OAuth2 token validator with JWKS caching.
pub struct GoogleTokenValidator {
    cached: RwLock<Option<CachedKeys>>,
    jwks_url: String,
    http_client: Option<reqwest::Client>,
}

impl GoogleTokenValidator {
    /// Create a new validator that fetches keys from the given JWKS URL.
    pub fn new(jwks_url: String) -> Self {
        Self {
            cached: RwLock::new(None),
            jwks_url,
            http_client: Some(reqwest::Client::new()),
        }
    }

    /// Create a validator with pre-loaded keys (for testing).
    #[cfg(test)]
    pub fn with_static_keys(keys: Vec<Jwk>) -> Self {
        Self {
            cached: RwLock::new(Some(CachedKeys {
                keys,
                fetched_at: Instant::now(),
            })),
            jwks_url: String::new(),
            http_client: None,
        }
    }

    /// Validate a Google token.
    ///
    /// Tries JWT validation first; falls back to opaque access token validation
    /// via userinfo if the token is not a valid JWT.
    async fn validate_token(
        &self,
        token: &str,
        audience: &str,
        domain: &str,
    ) -> Result<AuthenticatedUser, AuthError> {
        if decode_header(token).is_ok() {
            return self.validate_jwt(token, audience, domain).await;
        }

        log::debug!("Token is not a JWT, validating as access token via userinfo");
        self.validate_access_token(token, domain).await
    }

    /// Validate a Google ID token (JWT).
    async fn validate_jwt(
        &self,
        token: &str,
        audience: &str,
        domain: &str,
    ) -> Result<AuthenticatedUser, AuthError> {
        let header = decode_header(token).map_err(|e| AuthError::InvalidFormat(e.to_string()))?;
        let kid = header
            .kid
            .ok_or_else(|| AuthError::InvalidFormat("missing kid in JWT header".to_string()))?;

        let key = self.find_key(&kid).await?;
        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|e| AuthError::InvalidSignature(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[audience]);
        validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);

        let token_data =
            decode::<GoogleClaims>(token, &decoding_key, &validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::Expired,
                    jsonwebtoken::errors::ErrorKind::InvalidAudience => AuthError::InvalidAudience,
                    jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                        AuthError::InvalidFormat("invalid issuer".to_string())
                    }
                    _ => AuthError::InvalidSignature(e.to_string()),
                }
            })?;

        let claims = token_data.claims;
        let email = claims.email.ok_or(AuthError::MissingEmail)?;

        if claims.email_verified != Some(true) {
            return Err(AuthError::InvalidFormat(
                "email is not verified".to_string(),
            ));
        }

        Self::check_domain(&email, claims.hd, domain)?;

        Ok(AuthenticatedUser {
            email,
            subject: claims.sub,
        })
    }

    /// Validate an opaque Google access token via the userinfo endpoint.
    async fn validate_access_token(
        &self,
        token: &str,
        domain: &str,
    ) -> Result<AuthenticatedUser, AuthError> {
        let client = self.http_client.as_ref().ok_or_else(|| {
            AuthError::InvalidFormat("no HTTP client for access token validation".to_string())
        })?;

        let response = client
            .get(GOOGLE_USERINFO_URL)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetchError(format!("userinfo request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(AuthError::InvalidSignature(format!(
                "Google userinfo rejected token (HTTP {})",
                response.status()
            )));
        }

        let info: UserInfoResponse = response.json().await.map_err(|e| {
            AuthError::InvalidFormat(format!("userinfo response parse failed: {e}"))
        })?;

        let email = info.email.ok_or(AuthError::MissingEmail)?;

        if info.email_verified != Some(true) {
            return Err(AuthError::InvalidFormat(
                "email is not verified".to_string(),
            ));
        }

        Self::check_domain(&email, info.hd, domain)?;

        log::info!("Access token validated via userinfo for {email}");

        Ok(AuthenticatedUser {
            email,
            subject: info.sub,
        })
    }

    /// Verify that the user's domain matches the configured domain.
    fn check_domain(email: &str, hd: Option<String>, domain: &str) -> Result<(), AuthError> {
        if domain.is_empty() {
            return Ok(());
        }

        let user_domain = hd.unwrap_or_else(|| email.rsplit('@').next().unwrap_or("").to_string());

        if user_domain != domain {
            return Err(AuthError::InvalidDomain {
                domain: user_domain,
                expected: domain.to_string(),
            });
        }

        Ok(())
    }

    /// Find a key by `kid`, fetching/refreshing the cache as needed.
    async fn find_key(&self, kid: &str) -> Result<Jwk, AuthError> {
        if let Some(key) = self.lookup_cached(kid) {
            return Ok(key);
        }

        if self.http_client.is_some() {
            self.refresh_keys().await?;
            if let Some(key) = self.lookup_cached(kid) {
                return Ok(key);
            }
        }

        Err(AuthError::NoMatchingKey(kid.to_string()))
    }

    fn lookup_cached(&self, kid: &str) -> Option<Jwk> {
        let cache = self.cached.read().ok()?;
        let cached = cache.as_ref()?;

        if self.http_client.is_some() && cached.fetched_at.elapsed() > JWKS_CACHE_TTL {
            return None;
        }

        cached.keys.iter().find(|k| k.kid == kid).cloned()
    }

    async fn refresh_keys(&self) -> Result<(), AuthError> {
        let client = self.http_client.as_ref().ok_or_else(|| {
            AuthError::JwksFetchError("no HTTP client (static keys mode)".to_string())
        })?;

        let response: JwksResponse = client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?
            .json()
            .await
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?;

        let mut cache = self
            .cached
            .write()
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?;

        *cache = Some(CachedKeys {
            keys: response.keys,
            fetched_at: Instant::now(),
        });

        Ok(())
    }
}

impl TokenValidator for GoogleTokenValidator {
    fn validate(
        &self,
        token: &str,
        config: &AuthConfig,
    ) -> Pin<Box<dyn Future<Output = Result<AuthenticatedUser, AuthError>> + Send + '_>> {
        let token = token.to_string();
        let audience = config.audience.clone();
        let domain = config.domain.clone();
        Box::pin(async move { self.validate_token(&token, &audience, &domain).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    // Pre-generated 2048-bit RSA key pair for testing only.
    const TEST_RSA_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC6MDs7tbKS902a
XXEsFPSjdTC3S2bIj6ErdfrsOircv35PTckf3QhhwTwCDEp3raYC/bHDSYKytzwy
yLIBCyBmOZYraIq+A/nHawoVNNyDojwRO0Rg14gdsFG8hGt0KZR+BuETDU6uOzpW
3vKp3h8ZeNHgbPkt9Eyq/b6jJrHTwLhSZe9N1s282NV1FSrY0sGGZS54cb0e3Mf6
vqh8q8fJe3l5OZYeg6S+/kNFFctYURu3NAzd0mVB2X8jWujmP1J4G5L0NLmmCmob
f5yohzlNNb3jxCglgI07X3aEXbLxj+GLjIuWlnH38QxG/1x4kenrOYVK5oGuAMwh
Qxi5TgZfAgMBAAECggEAS+AG64zewpSkm+Ezkx7RGWHTIgdI0jfyGseAI8+Kkx44
p7HP4jvNxCSew0jl+gKakkJ3tFlbOWCB2EJAhDtYD0CtiXAXhlsRaeqdl8nMiZpO
N0l7UqnS7yJhFN2z+olNWdSM2ZpFM6ywWCGQK5h4/QTnJrnSDB+wNMimbU+CDYQc
xvgpyVyogDTAZzIMTtvc2imaCjy8HLKan3nNDj1mZT5qMJl/QOVqd75vbkthsp/B
T7++3ITMaivMTpVMH9XI3hyd+CVT3jKxqWSXzY/m0P3InwB6kdHa6/cPfANEIH/w
d1Iyjwn9mDVavHqPziJUrt8Kn5TZ83KN29Gd/zxFWQKBgQDb8JWHKFCZl4eH8MdT
7UoSgUtUhrqkRWk7esYzcAKU+RZU6TeIJltZ01rIzrXfcoHsTMaEb18zcEME6H2o
cUb1GvoisbEqY/qy+3BowMaMdGDg98Y3g7dlrH/BJ8hLTU5FjW7m0XdAnuMIuDMW
FHL1/HYUAmC2SqNOMlTzDeSuuQKBgQDYtwTCaBIkIUedpBqU/QGeojH9cySoo7WM
33mBDf+eU5E3fk/NcmDSynb3naIYBNUpi3BtOJHA1P5L2j0JMj4pv3FP7kjqnZJl
7pKdmx7tx2xwfDKHdrg5esYLUnbCXTk/GUnixnLtyeJR2jF4IxVMIobePQybYI+6
/3ItWU0R1wKBgHTgzeVsVCC6+MgR+Sstf16EHQ8HJeokBL8aCHfPP2ABWo+2+867
a3I5shXiW54p0MdNKXW5ZaMFNmhGUHiR8f5Q3rpPKXH4fYJdwie4wgpj0hPbOBfK
REygtadkx7jUlRK7DUNV7wSFKus4T9Wc+lakWe9aMCDPWycz8hbTvEHpAoGBANOD
bWXA5VPWF2vIqxkXBumpLFlOdE0T2zIvOwu2efIxZd5frcu7Ar05Vnu+omIG9XWi
3ov7VmZ6e+fUjRXYr8tXSmTVEN3MBQLvorGooLs6lKAE19xXBt8y8PBEAB0bl6/6
Ip7vSWTEUdvJtdanhzXTzQZDV3ae/ClrACk6q3npAoGAcIAwBJazKsk/ZqvjTjjQ
pjxTpsa+XEG97A8s0dJniyReDkr1DlP4my4mf9ioo/vMjDPqCz6vThlC/e+fBq+Y
w6RhjrMdqz8mlMuvov67XyoebzSx8earuR5ANFGCuExhPGNRNMTwO7Al6H6rGFhN
kooeTEzEZMJu3/AKnRMd2NY=
-----END PRIVATE KEY-----";

    const TEST_RSA_N: &str = "ujA7O7WykvdNml1xLBT0o3Uwt0tmyI-hK3X67Doq3L9-T03JH90IYcE8AgxKd62mAv2xw0mCsrc8MsiyAQsgZjmWK2iKvgP5x2sKFTTcg6I8ETtEYNeIHbBRvIRrdCmUfgbhEw1Orjs6Vt7yqd4fGXjR4Gz5LfRMqv2-oyax08C4UmXvTdbNvNjVdRUq2NLBhmUueHG9HtzH-r6ofKvHyXt5eTmWHoOkvv5DRRXLWFEbtzQM3dJlQdl_I1ro5j9SeBuS9DS5pgpqG3-cqIc5TTW948QoJYCNO192hF2y8Y_hi4yLlpZx9_EMRv9ceJHp6zmFSuaBrgDMIUMYuU4GXw";
    const TEST_RSA_E: &str = "AQAB";
    const TEST_KID: &str = "test-kid-1";
    const TEST_AUDIENCE: &str = "test-client-id.apps.googleusercontent.com";
    const TEST_DOMAIN: &str = "banyan.com";

    #[derive(Debug, Serialize)]
    struct TestClaims {
        sub: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        email_verified: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hd: Option<String>,
        iss: String,
        aud: String,
        exp: u64,
        iat: u64,
    }

    fn test_cache() -> GoogleTokenValidator {
        GoogleTokenValidator::with_static_keys(vec![Jwk {
            kid: TEST_KID.to_string(),
            n: TEST_RSA_N.to_string(),
            e: TEST_RSA_E.to_string(),
        }])
    }

    fn now_epoch() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn valid_claims() -> TestClaims {
        let now = now_epoch();
        TestClaims {
            sub: "sub_123".to_string(),
            email: Some("alice@banyan.com".to_string()),
            email_verified: Some(true),
            hd: Some("banyan.com".to_string()),
            iss: "https://accounts.google.com".to_string(),
            aud: TEST_AUDIENCE.to_string(),
            exp: now + 3600,
            iat: now,
        }
    }

    fn sign_token(claims: &TestClaims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(TEST_KID.to_string());
        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_PEM.as_bytes()).unwrap();
        encode(&header, claims, &key).unwrap()
    }

    #[tokio::test]
    async fn test_validate_valid_jwt() {
        let cache = test_cache();
        let token = sign_token(&valid_claims());
        let user = cache
            .validate_token(&token, TEST_AUDIENCE, TEST_DOMAIN)
            .await
            .unwrap();
        assert_eq!(user.email, "alice@banyan.com");
        assert_eq!(user.subject, "sub_123");
    }

    #[tokio::test]
    async fn test_validate_expired_jwt() {
        let cache = test_cache();
        let mut claims = valid_claims();
        claims.exp = now_epoch() - 3600;
        let token = sign_token(&claims);
        let result = cache
            .validate_token(&token, TEST_AUDIENCE, TEST_DOMAIN)
            .await;
        assert!(matches!(result, Err(AuthError::Expired)));
    }

    #[tokio::test]
    async fn test_validate_wrong_audience() {
        let cache = test_cache();
        let mut claims = valid_claims();
        claims.aud = "wrong-audience".to_string();
        let token = sign_token(&claims);
        let result = cache
            .validate_token(&token, TEST_AUDIENCE, TEST_DOMAIN)
            .await;
        assert!(matches!(result, Err(AuthError::InvalidAudience)));
    }

    #[tokio::test]
    async fn test_validate_wrong_domain() {
        let cache = test_cache();
        let mut claims = valid_claims();
        claims.hd = Some("other.com".to_string());
        let token = sign_token(&claims);
        let result = cache
            .validate_token(&token, TEST_AUDIENCE, TEST_DOMAIN)
            .await;
        assert!(matches!(
            result,
            Err(AuthError::InvalidDomain { ref domain, .. }) if domain == "other.com"
        ));
    }

    #[tokio::test]
    async fn test_validate_missing_email() {
        let cache = test_cache();
        let mut claims = valid_claims();
        claims.email = None;
        claims.hd = None;
        let token = sign_token(&claims);
        let result = cache.validate_token(&token, TEST_AUDIENCE, "").await;
        assert!(matches!(result, Err(AuthError::MissingEmail)));
    }

    #[tokio::test]
    async fn test_validate_empty_domain_allows_any() {
        let cache = test_cache();
        let mut claims = valid_claims();
        claims.hd = Some("any-domain.com".to_string());
        let token = sign_token(&claims);
        let result = cache.validate_token(&token, TEST_AUDIENCE, "").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_domain_empty_allows_any() {
        assert!(GoogleTokenValidator::check_domain("a@b.com", None, "").is_ok());
    }

    #[test]
    fn test_check_domain_hd_takes_precedence() {
        assert!(GoogleTokenValidator::check_domain(
            "alice@other.com",
            Some("banyan.com".to_string()),
            "banyan.com"
        )
        .is_ok());
    }

    #[test]
    fn test_check_domain_fallback_from_email() {
        assert!(GoogleTokenValidator::check_domain("alice@banyan.com", None, "banyan.com").is_ok());
    }
}
