use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug)]
pub enum UserError {
    DatabaseError(sqlx::Error),
    PasswordHashError,
    UsernameExists,
    UserNotFound,
    InvalidPassword,
    InvalidUsername,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::DatabaseError(e) => write!(f, "Database error: {}", e),
            UserError::PasswordHashError => write!(f, "Failed to hash password"),
            UserError::UsernameExists => write!(f, "Username already exists"),
            UserError::UserNotFound => write!(f, "User not found"),
            UserError::InvalidPassword => write!(f, "Invalid password"),
            UserError::InvalidUsername => write!(f, "Invalid username"),
        }
    }
}

impl std::error::Error for UserError {}

pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_user(&self, username: &str, password: &str) -> Result<User, UserError> {
        if username.len() < 3 || username.len() > 50 {
            return Err(UserError::InvalidUsername);
        }
        if password.len() < 6 {
            return Err(UserError::InvalidPassword);
        }

        let password_hash = hash_password(password)?;
        let user_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&user_id)
        .bind(username)
        .bind(&password_hash)
        .bind(&now)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(User {
                id: user_id,
                username: username.to_string(),
                password_hash,
                created_at: now,
            }),
            Err(sqlx::Error::Database(ref db_err)) if db_err.message().contains("UNIQUE") => {
                Err(UserError::UsernameExists)
            }
            Err(e) => Err(UserError::DatabaseError(e)),
        }
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, UserError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(UserError::DatabaseError)
    }

    pub async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, UserError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(UserError::DatabaseError)
    }

    pub fn verify_password(&self, user: &User, password: &str) -> Result<bool, UserError> {
        verify_password(password, &user.password_hash)
    }
}

fn hash_password(password: &str) -> Result<String, UserError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| UserError::PasswordHashError)
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, UserError> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| UserError::InvalidPassword)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
