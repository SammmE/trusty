// Session storage for login credentials
const LOGIN_PASSWORD_KEY = 'trusty_login_password';
const USE_LOGIN_PASSWORD_KEY = 'trusty_use_login_password';

export function setLoginPassword(password: string): void {
  sessionStorage.setItem(LOGIN_PASSWORD_KEY, password);
}

export function getLoginPassword(): string | null {
  return sessionStorage.getItem(LOGIN_PASSWORD_KEY);
}

export function clearLoginPassword(): void {
  sessionStorage.removeItem(LOGIN_PASSWORD_KEY);
}

export function setUseLoginPassword(use: boolean): void {
  sessionStorage.setItem(USE_LOGIN_PASSWORD_KEY, use ? 'true' : 'false');
}

export function getUseLoginPassword(): boolean {
  return sessionStorage.getItem(USE_LOGIN_PASSWORD_KEY) === 'true';
}

export function clearSession(): void {
  clearLoginPassword();
  sessionStorage.removeItem(USE_LOGIN_PASSWORD_KEY);
}
