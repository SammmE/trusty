export class CryptoError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CryptoError';
  }
}

const PBKDF2_ITERATIONS = 100000;
const SALT_LENGTH = 16;
const IV_LENGTH = 12;

export async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  const encoder = new TextEncoder();
  const passwordBuffer = encoder.encode(password);

  const baseKey = await crypto.subtle.importKey(
    'raw',
    passwordBuffer,
    'PBKDF2',
    false,
    ['deriveBits', 'deriveKey']
  );

  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: salt,
      iterations: PBKDF2_ITERATIONS,
      hash: 'SHA-256',
    },
    baseKey,
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt']
  );
}

export async function encryptFile(
  file: File,
  password: string
): Promise<{ encryptedBlob: Blob; salt: Uint8Array; iv: Uint8Array }> {
  try {
    const salt = crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
    const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));

    const key = await deriveKey(password, salt);

    const fileBuffer = await file.arrayBuffer();

    const encryptedBuffer = await crypto.subtle.encrypt(
      {
        name: 'AES-GCM',
        iv: iv,
      },
      key,
      fileBuffer
    );

    const combinedBuffer = new Uint8Array(
      SALT_LENGTH + IV_LENGTH + encryptedBuffer.byteLength
    );
    combinedBuffer.set(salt, 0);
    combinedBuffer.set(iv, SALT_LENGTH);
    combinedBuffer.set(new Uint8Array(encryptedBuffer), SALT_LENGTH + IV_LENGTH);

    const encryptedBlob = new Blob([combinedBuffer], { type: 'application/octet-stream' });

    return { encryptedBlob, salt, iv };
  } catch (error) {
    throw new CryptoError(`Encryption failed: ${error}`);
  }
}

export async function decryptFile(
  encryptedBlob: Blob,
  password: string,
  originalFileName: string
): Promise<Blob> {
  try {
    const encryptedBuffer = await encryptedBlob.arrayBuffer();
    const encryptedArray = new Uint8Array(encryptedBuffer);

    if (encryptedArray.byteLength < SALT_LENGTH + IV_LENGTH) {
      throw new CryptoError('Invalid encrypted file format');
    }

    const salt = encryptedArray.slice(0, SALT_LENGTH);
    const iv = encryptedArray.slice(SALT_LENGTH, SALT_LENGTH + IV_LENGTH);
    const ciphertext = encryptedArray.slice(SALT_LENGTH + IV_LENGTH);

    const key = await deriveKey(password, salt);

    const decryptedBuffer = await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: iv,
      },
      key,
      ciphertext
    );

    const mimeType = guessMimeType(originalFileName);
    return new Blob([decryptedBuffer], { type: mimeType });
  } catch (error) {
    if (error instanceof CryptoError) {
      throw error;
    }
    throw new CryptoError('Decryption failed - wrong password or corrupted file');
  }
}

function guessMimeType(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();
  const mimeTypes: Record<string, string> = {
    pdf: 'application/pdf',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    txt: 'text/plain',
    json: 'application/json',
    zip: 'application/zip',
    mp4: 'video/mp4',
    mp3: 'audio/mpeg',
  };
  return mimeTypes[ext || ''] || 'application/octet-stream';
}

export function downloadDecryptedFile(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
