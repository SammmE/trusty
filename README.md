# Trusty - Encrypted File Storage System

## Overview

Trusty is a secure, multi-user encrypted file storage system implementing the "Bucket Model" where each user has an isolated directory for their files. The system uses client-side encryption (AES-GCM-256) before files are uploaded, making the backend a "blind store" that never sees unencrypted content.

## Architecture

- **Backend**: Rust (Axum + SQLite + JWT)
- **Frontend**: Svelte 5 + TypeScript + Tailwind CSS
- **Encryption**: Web Crypto API (AES-GCM-256, PBKDF2)
- **Storage**: `./storage/<user_uuid>/<file_uuid>.bin`

## Features

✅ **Multi-user authentication** (username/password with Argon2 hashing)  
✅ **User bucket creation** on signup  
✅ **Client-side file encryption** before upload  
✅ **Blind backend storage** (receives pre-encrypted blobs)  
✅ **File metadata indexing** (search by filename)  
✅ **Secure file download** with password-based decryption  
✅ **File deletion** with cleanup  
✅ **User isolation** via JWT-based access control  
✅ **OpenAPI/Swagger documentation**

## Setup

### Prerequisites

- Rust (1.70+)
- Node.js (18+)
- SQLite

### Installation

1. **Clone and setup environment**:

```bash
cp .env.example .env
# Edit .env and set a secure JWT_SECRET
```

2. **Build frontend**:

```bash
cd frontend
npm install
npm run build
cd ..
```

3. **Run the server**:

```bash
cargo run
```

The server will start on `http://localhost:3000`. The database will be created automatically if it doesn't exist, and migrations will be applied.

## Usage

### Sign Up

1. Navigate to `http://localhost:3000`
2. Click "Don't have an account? Sign up"
3. Enter username and password
4. Your user bucket will be created automatically at `./storage/<user_id>/`

⚠️ **WARNING**: Your password is used to derive the encryption key. If you lose it, your files cannot be recovered.

### Upload Files

1. Click "Upload File" button
2. Select a file (or drag and drop)
3. Enter an encryption password
4. File will be encrypted client-side and uploaded

### Download Files

1. Click the "⋮" menu or right-click on a file
2. Select "Download"
3. Enter the decryption password
4. File will be downloaded and decrypted in your browser

### Search Files

Use the search bar to filter files by name (searches plaintext metadata).

## API Endpoints

### Authentication

- `POST /api/auth/signup` - Create new user
- `POST /api/auth/login` - Authenticate user
- `GET /api/auth/me` - Get current user info

### Files

- `GET /api/files` - List files (with search/sort)
- `POST /api/files/upload` - Upload encrypted file (multipart)
- `GET /api/files/:id/download` - Download encrypted file
- `DELETE /api/files/:id` - Delete file

### Documentation

- `GET /swagger-ui` - Interactive API documentation
- `GET /api/openapi.json` - OpenAPI specification

## Security

- **Password Hashing**: Argon2 (industry standard, GPU-resistant)
- **JWT Tokens**: Ed25519 signatures, 24-hour expiration
- **Client-Side Encryption**: AES-GCM-256 with PBKDF2 key derivation (100,000 iterations)
- **User Isolation**: All file operations verify ownership via JWT user_id
- **Blind Storage**: Backend never sees unencrypted file content

## File Structure

```
trusty/
├── src/
│   ├── main.rs           # Server setup, routes, CORS
│   ├── auth.rs           # JWT auth, signup, login
│   ├── user.rs           # User model, repository, Argon2
│   ├── filemanager.rs    # File CRUD, upload/download
│   └── static_files.rs   # Frontend SPA serving
├── frontend/
│   ├── src/
│   │   ├── App.svelte              # Main app component
│   │   ├── lib/
│   │   │   ├── api.ts              # API client
│   │   │   ├── crypto.ts           # Web Crypto API wrappers
│   │   │   └── components/
│   │   │       ├── LoginForm.svelte
│   │   │       ├── FileTable.svelte
│   │   │       └── FileUploadModal.svelte
│   │   └── ...
│   └── ...
├── migrations/
│   └── 20260217000000_initial_schema.sql
├── storage/              # User buckets (created at runtime)
│   └── <user_uuid>/
│       └── <file_uuid>.bin
├── Cargo.toml
└── .env
```

## Database Schema

### users

| Column        | Type | Description                |
| ------------- | ---- | -------------------------- |
| id            | TEXT | UUID (primary key)         |
| username      | TEXT | Unique username            |
| password_hash | TEXT | Argon2 hash                |
| created_at    | TEXT | ISO 8601 timestamp         |

### files

| Column       | Type    | Description                    |
| ------------ | ------- | ------------------------------ |
| id           | TEXT    | UUID (primary key)             |
| user_id      | TEXT    | Foreign key to users.id        |
| original_name| TEXT    | Plaintext filename (searchable)|
| mime_type    | TEXT    | Content type                   |
| size_bytes   | INTEGER | File size                      |
| is_encrypted | INTEGER | Boolean (always 1)             |
| storage_path | TEXT    | Relative path to blob          |
| created_at   | TEXT    | ISO 8601 timestamp             |

## Environment Variables

```
DATABASE_URL=sqlite:trusty.db
JWT_SECRET=your-256-bit-secret
STORAGE_ROOT=./storage
PORT=3000
MAX_FILE_SIZE_MB=100
```

## Development

### Run backend in dev mode:

```bash
cargo watch -x run
```

### Run frontend dev server:

```bash
cd frontend
npm run dev
```

(Note: Update `API_BASE` in `frontend/src/lib/api.ts` to `http://localhost:3000` for dev)

### Check for errors:

```bash
cargo check
cargo clippy
```

### Run tests:

```bash
cargo test
```

## Limitations

- No file sharing between users
- No folder/directory structure (flat file list)
- No file versioning
- No server-side encryption (client-side only)
- No pagination (will be slow with large file counts)
- No file rename operation

## Future Enhancements

- File sharing with expiring links
- Folder support
- File versioning
- Thumbnails/previews for images
- Rate limiting and quotas
- Email verification
- Multi-factor authentication
- Mobile app (React Native with Web Crypto API)

## License

MIT
