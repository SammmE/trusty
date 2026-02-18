use std::fmt::Display;

use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use ed25519_dalek::SigningKey;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use pkcs8::EncodePrivateKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::user::{CreateUserRequest, UserRepository, UserResponse};
use crate::AppState;

pub struct Keys {
    pub(crate) encoding: EncodingKey,
    pub(crate) decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        // 1. Deterministically derive the Ed25519 key pair from the secret
        let mut seed = [0u8; 32];
        let len = secret.len().min(32);
        seed[..len].copy_from_slice(&secret[..len]);

        // If secret is short, repeat pattern to fill 32 bytes
        if len < 32 {
            for i in len..32 {
                seed[i] = seed[i % len];
            }
        }

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        // 2. Prepare the keys for jsonwebtoken

        // ENCODING: Must be PKCS#8 DER
        let private_key_der = signing_key
            .to_pkcs8_der()
            .expect("Failed to encode private key to PKCS#8");

        // DECODING: Use raw bytes (32 bytes) to avoid SPKI formatting issues
        let public_key_bytes = verifying_key.as_bytes();

        Self {
            encoding: EncodingKey::from_ed_der(private_key_der.as_bytes()),
            decoding: DecodingKey::from_ed_der(public_key_bytes),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub user_id: String,
    pub username: String,
    pub exp: usize,
}

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Claims {{ user_id: {}, username: {}, exp: {} }}",
            self.user_id, self.username, self.exp
        )
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthBody {
    pub access_token: String,
    pub token_type: String,
    pub user: UserResponse,
}

impl AuthBody {
    pub fn new(access_token: String, user: UserResponse) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
            user,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    UsernameExists,
    InvalidUsername,
    InvalidPassword,
    StorageError,
    InternalError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::UsernameExists => (StatusCode::BAD_REQUEST, "Username already exists"),
            AuthError::InvalidUsername => (
                StatusCode::BAD_REQUEST,
                "Invalid username (must be 3-50 characters)",
            ),
            AuthError::InvalidPassword => (
                StatusCode::BAD_REQUEST,
                "Invalid password (must be at least 6 characters)",
            ),
            AuthError::StorageError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create user storage",
            ),
            AuthError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            // 1. Extract the header
            let authorization = parts
                .headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .ok_or(AuthError::MissingCredentials)?;

            // 2. Strip "Bearer " prefix
            let token = authorization
                .strip_prefix("Bearer ")
                .ok_or(AuthError::InvalidToken)?;

            // 3. Decode & Validate
            let keys = &crate::KEYS;
            let mut validation = Validation::new(Algorithm::EdDSA);
            validation.validate_exp = true;
            // Ensure the validation algorithms match the key type
            validation.algorithms = vec![Algorithm::EdDSA];

            let token_data = decode::<Claims>(token, &keys.decoding, &validation).map_err(|e| {
                eprintln!("Token decoding error: {:?}", e);
                AuthError::InvalidToken
            })?;

            Ok(token_data.claims)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/signup",
    request_body = CreateUserRequest,
    tag = "auth",
    responses(
        (status = 201, description = "User created successfully", body = AuthBody),
        (status = 400, description = "Invalid input or username already exists"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<AuthBody>), AuthError> {
    let user_repo = UserRepository::new(state.db_pool.clone());

    let user = user_repo
        .create_user(&payload.username, &payload.password)
        .await
        .map_err(|e| match e {
            crate::user::UserError::UsernameExists => AuthError::UsernameExists,
            crate::user::UserError::InvalidUsername => AuthError::InvalidUsername,
            crate::user::UserError::InvalidPassword => AuthError::InvalidPassword,
            _ => AuthError::InternalError,
        })?;

    let bucket_path = state.storage_root.join(&user.id);
    tokio::fs::create_dir_all(&bucket_path)
        .await
        .map_err(|_| AuthError::StorageError)?;

    let claims = Claims {
        user_id: user.id.clone(),
        username: user.username.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let header = Header::new(Algorithm::EdDSA);
    let token = encode(&header, &claims, &crate::KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    let user_response: UserResponse = user.into();
    Ok((
        StatusCode::CREATED,
        Json(AuthBody::new(token, user_response)),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    tag = "auth",
    responses(
        (status = 200, description = "Login successful", body = AuthBody),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthBody>, AuthError> {
    let user_repo = UserRepository::new(state.db_pool.clone());

    let user = user_repo
        .find_by_username(&payload.username)
        .await
        .map_err(|_| AuthError::InternalError)?
        .ok_or(AuthError::WrongCredentials)?;

    let is_valid = user_repo
        .verify_password(&user, &payload.password)
        .map_err(|_| AuthError::InternalError)?;

    if !is_valid {
        return Err(AuthError::WrongCredentials);
    }

    let claims = Claims {
        user_id: user.id.clone(),
        username: user.username.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let header = Header::new(Algorithm::EdDSA);
    let token = encode(&header, &claims, &crate::KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    let user_response: UserResponse = user.into();
    Ok(Json(AuthBody::new(token, user_response)))
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user info", body = Claims),
        (status = 400, description = "Invalid or missing token"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn me(claims: Claims) -> Result<Json<Claims>, AuthError> {
    Ok(Json(claims))
}
