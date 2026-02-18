const API_BASE = 'http://localhost:3000';

export interface User {
  id: string;
  username: string;
  created_at: string;
}

export interface AuthResponse {
  access_token: string;
  token_type: string;
  user: User;
}

export interface FileMetadata {
  id: string;
  original_name: string;
  mime_type: string;
  size_bytes: number;
  created_at: string;
}

export interface FileListResponse {
  files: FileMetadata[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface SignupRequest {
  username: string;
  password: string;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public body?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

function getAuthToken(): string | null {
  return sessionStorage.getItem('auth_token') || localStorage.getItem('auth_token');
}

export function setAuthToken(token: string): void {
  sessionStorage.setItem('auth_token', token);
  localStorage.setItem('auth_token', token);
}

export function clearAuthToken(): void {
  sessionStorage.removeItem('auth_token');
  localStorage.removeItem('auth_token');
}

export function isAuthenticated(): boolean {
  return getAuthToken() !== null;
}

async function apiRequest<T>(
  method: string,
  path: string,
  body?: any,
  headers?: Record<string, string>
): Promise<T> {
  const token = getAuthToken();
  const requestHeaders: Record<string, string> = {
    ...headers,
  };

  if (token) {
    requestHeaders['Authorization'] = `Bearer ${token}`;
  }

  if (body && !(body instanceof FormData)) {
    requestHeaders['Content-Type'] = 'application/json';
  }

  const response = await fetch(`${API_BASE}${path}`, {
    method,
    headers: requestHeaders,
    body: body instanceof FormData ? body : body ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    const errorBody = await response.json().catch(() => ({}));
    throw new ApiError(
      errorBody.error || `HTTP ${response.status}`,
      response.status,
      errorBody
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

export async function signup(username: string, password: string): Promise<AuthResponse> {
  const response = await apiRequest<AuthResponse>('POST', '/api/auth/signup', {
    username,
    password,
  });
  setAuthToken(response.access_token);
  return response;
}

export async function login(username: string, password: string): Promise<AuthResponse> {
  const response = await apiRequest<AuthResponse>('POST', '/api/auth/login', {
    username,
    password,
  });
  setAuthToken(response.access_token);
  return response;
}

export async function logout(): Promise<void> {
  clearAuthToken();
}

export async function getCurrentUser(): Promise<any> {
  return apiRequest('GET', '/api/auth/me');
}

export async function listFiles(
  searchQuery?: string,
  sort?: string,
  direction?: string,
  page?: number,
  pageSize?: number
): Promise<FileListResponse> {
  const params = new URLSearchParams();
  if (searchQuery) params.append('q', searchQuery);
  if (sort) params.append('sort', sort);
  if (direction) params.append('direction', direction);
  if (page) params.append('page', page.toString());
  if (pageSize) params.append('page_size', pageSize.toString());

  const queryString = params.toString();
  const path = queryString ? `/api/files?${queryString}` : '/api/files';

  return apiRequest('GET', path);
}

export async function uploadFile(
  encryptedBlob: Blob,
  metadata: {
    original_name: string;
    mime_type: string;
    size_bytes: number;
    client_encryption_algo: string;
  }
): Promise<FileMetadata> {
  const formData = new FormData();
  formData.append('file', encryptedBlob);
  formData.append('metadata', JSON.stringify(metadata));

  return apiRequest('POST', '/api/files/upload', formData);
}

export async function downloadFile(fileId: string): Promise<Blob> {
  const token = getAuthToken();
  const response = await fetch(`${API_BASE}/api/files/${fileId}/download`, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });

  if (!response.ok) {
    const errorBody = await response.json().catch(() => ({}));
    throw new ApiError(
      errorBody.error || `HTTP ${response.status}`,
      response.status,
      errorBody
    );
  }

  return response.blob();
}

export async function deleteFile(fileId: string): Promise<void> {
  return apiRequest('DELETE', `/api/files/${fileId}`);
}
