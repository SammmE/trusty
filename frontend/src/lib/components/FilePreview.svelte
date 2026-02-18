<script lang="ts">
	import { onMount } from 'svelte';
	import { downloadFile, type FileMetadata, type ApiError } from '$lib/api';
	import { decryptFile, CryptoError } from '$lib/crypto';
	import { getLoginPassword, getUseLoginPassword } from '$lib/session';
	import hljs from 'highlight.js';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Download, X } from 'lucide-svelte';

	interface Props {
		open: boolean;
		file: FileMetadata | null;
		onClose: () => void;
	}

	let { open = $bindable(), file, onClose }: Props = $props();

	let decryptedBlob = $state<Blob | null>(null);
	let objectUrl = $state<string | null>(null);
	let loading = $state(false);
	let error = $state('');
	let showPasswordDialog = $state(false);
	let password = $state('');
	let passwordError = $state('');
	let fileType = $state<'image' | 'video' | 'audio' | 'pdf' | 'text' | 'unknown'>('unknown');
	let textContent = $state('');
	let codeElement = $state<HTMLElement | null>(null);

	function getLanguageFromFileName(fileName: string): string {
		const ext = fileName.toLowerCase().split('.').pop() || '';
		
		const languageMap: Record<string, string> = {
			'js': 'javascript',
			'jsx': 'javascript',
			'ts': 'typescript',
			'tsx': 'typescript',
			'py': 'python',
			'rb': 'ruby',
			'java': 'java',
			'cpp': 'cpp',
			'c': 'c',
			'h': 'c',
			'hpp': 'cpp',
			'cs': 'csharp',
			'php': 'php',
			'go': 'go',
			'rs': 'rust',
			'swift': 'swift',
			'kt': 'kotlin',
			'html': 'html',
			'css': 'css',
			'scss': 'scss',
			'sass': 'sass',
			'less': 'less',
			'json': 'json',
			'xml': 'xml',
			'yaml': 'yaml',
			'yml': 'yaml',
			'toml': 'toml',
			'ini': 'ini',
			'sh': 'bash',
			'bash': 'bash',
			'zsh': 'bash',
			'fish': 'bash',
			'sql': 'sql',
			'md': 'markdown',
			'markdown': 'markdown',
			'dockerfile': 'dockerfile',
			'makefile': 'makefile'
		};
		
		return languageMap[ext] || 'plaintext';
	}

	function getFileType(fileName: string, mimeType: string): 'image' | 'video' | 'audio' | 'pdf' | 'text' | 'unknown' {
		const ext = fileName.toLowerCase().split('.').pop();
		
		// Images
		if (['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg', 'ico'].includes(ext || '') || 
		    mimeType.startsWith('image/')) {
			return 'image';
		}
		
		// Videos
		if (['mp4', 'webm', 'ogg', 'mov', 'avi', 'mkv', 'wmv', 'flv', 'm4v'].includes(ext || '') || 
		    mimeType.startsWith('video/')) {
			return 'video';
		}
		
		// Audio
		if (['mp3', 'wav', 'ogg', 'flac', 'm4a', 'aac', 'wma', 'opus'].includes(ext || '') || 
		    mimeType.startsWith('audio/')) {
			return 'audio';
		}
		
		// PDF
		if (ext === 'pdf' || mimeType === 'application/pdf') {
			return 'pdf';
		}
		
		// Text/Code files
		if (['txt', 'md', 'markdown', 'json', 'xml', 'yaml', 'yml', 'toml', 'ini', 'config',
		     'js', 'ts', 'jsx', 'tsx', 'py', 'java', 'cpp', 'c', 'h', 'hpp', 'cs', 'php', 
		     'rb', 'go', 'rs', 'swift', 'kt', 'html', 'css', 'scss', 'sass', 'less',
		     'sh', 'bash', 'zsh', 'fish', 'log', 'csv', 'sql'].includes(ext || '') ||
		    mimeType.startsWith('text/') ||
		    mimeType === 'application/json' ||
		    mimeType === 'application/xml') {
			return 'text';
		}
		
		return 'unknown';
	}

	async function loadPreview() {
		if (!file) return;
		
		loading = true;
		error = '';
		
		// First try with login password if enabled
		if (getUseLoginPassword()) {
			const loginPwd = getLoginPassword();
			if (loginPwd) {
				await tryDecryptWithPassword(loginPwd);
				return;
			}
		}
		
		// Otherwise show password dialog
		loading = false;
		showPasswordDialog = true;
	}

	async function tryDecryptWithPassword(pwd: string) {
		if (!file) return;
		
		loading = true;
		error = '';
		passwordError = '';
		
		try {
			const encryptedBlob = await downloadFile(file.id);
			const decrypted = await decryptFile(encryptedBlob, pwd, file.original_name);
			
			decryptedBlob = decrypted;
			fileType = getFileType(file.original_name, file.mime_type);
			
			// Create object URL for preview
			if (objectUrl) {
				URL.revokeObjectURL(objectUrl);
			}
			objectUrl = URL.createObjectURL(decrypted);
			
			// For text files, read the content
			if (fileType === 'text') {
				const text = await decrypted.text();
				textContent = text;
				
				// Apply syntax highlighting after content is set
				setTimeout(() => {
					if (codeElement) {
						hljs.highlightElement(codeElement);
					}
				}, 0);
			}
			
			showPasswordDialog = false;
			password = '';
		} catch (err) {
			if (err instanceof CryptoError) {
				passwordError = 'Decryption failed - wrong password';
				if (!showPasswordDialog) {
					showPasswordDialog = true;
				}
			} else if (err instanceof Error) {
				const apiError = err as ApiError;
				error = apiError.body?.error || apiError.message || 'Failed to load preview';
			}
		} finally {
			loading = false;
		}
	}

	async function handlePasswordSubmit() {
		if (!password) {
			passwordError = 'Please enter a password';
			return;
		}
		await tryDecryptWithPassword(password);
	}

	function handleClose() {
		if (objectUrl) {
			URL.revokeObjectURL(objectUrl);
		}
		decryptedBlob = null;
		objectUrl = null;
		loading = false;
		error = '';
		showPasswordDialog = false;
		password = '';
		passwordError = '';
		textContent = '';
		codeElement = null;
		onClose();
	}

	function downloadDecryptedFile() {
		if (!decryptedBlob || !file) return;
		
		const url = URL.createObjectURL(decryptedBlob);
		const a = document.createElement('a');
		a.href = url;
		a.download = file.original_name;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
	}

	$effect(() => {
		if (open && file) {
			loadPreview();
		} else if (!open) {
			if (objectUrl) {
				URL.revokeObjectURL(objectUrl);
			}
			decryptedBlob = null;
			objectUrl = null;
			showPasswordDialog = false;
			password = '';
			passwordError = '';
			textContent = '';
			codeElement = null;
		}
	});
</script>

<Dialog bind:open>
	<DialogContent class="sm:max-w-[90vw] max-h-[90vh] overflow-hidden">
		<div class="flex flex-col max-h-[90vh]">
		<DialogHeader>
			<div class="flex items-center justify-between">
				<div class="flex-1 min-w-0">
					<DialogTitle class="truncate">{file?.original_name || 'Preview'}</DialogTitle>
					<DialogDescription>
						{file ? `${(file.size_bytes / 1024).toFixed(2)} KB` : ''}
					</DialogDescription>
				</div>
				<div class="flex items-center gap-2 ml-4">
					{#if decryptedBlob}
						<Button variant="outline" size="sm" onclick={downloadDecryptedFile}>
							<Download class="h-4 w-4 mr-2" />
							Download
						</Button>
					{/if}
				</div>
			</div>
		</DialogHeader>

		<div class="flex-1 overflow-auto">
			{#if loading}
				<div class="flex h-64 items-center justify-center">
					<div class="text-center">
						<div class="mb-4 h-8 w-8 animate-spin rounded-full border-4 border-gray-300 border-t-blue-600 mx-auto"></div>
						<p class="text-sm text-gray-600">Decrypting and loading preview...</p>
					</div>
				</div>
			{:else if error}
				<div class="flex h-64 items-center justify-center">
					<div class="rounded-md bg-red-50 p-4 max-w-md">
						<p class="text-sm text-red-800">{error}</p>
					</div>
				</div>
			{:else if showPasswordDialog}
				<div class="flex h-64 items-center justify-center">
					<div class="w-full max-w-md space-y-4 p-4">
						<div>
							<Label for="preview-password">Enter password to decrypt file</Label>
							<Input
								id="preview-password"
								type="password"
								bind:value={password}
								placeholder="Enter password"
								class="mt-2"
								onkeydown={(e) => e.key === 'Enter' && handlePasswordSubmit()}
							/>
						</div>
						
						{#if passwordError}
							<div class="rounded-md bg-red-50 p-3">
								<p class="text-sm text-red-800">{passwordError}</p>
							</div>
						{/if}
						
						<Button onclick={handlePasswordSubmit} class="w-full" disabled={!password}>
							Decrypt & Preview
						</Button>
					</div>
				</div>
			{:else if objectUrl}
				{#if fileType === 'image'}
					<div class="flex items-center justify-center bg-gray-50 rounded-lg p-4">
						<img src={objectUrl} alt={file?.original_name} class="max-w-full max-h-[70vh] object-contain" />
					</div>
				{:else if fileType === 'video'}
					<div class="flex items-center justify-center bg-black rounded-lg">
						<!-- svelte-ignore a11y_media_has_caption -->
						<video src={objectUrl} controls class="max-w-full max-h-[70vh]">
							Your browser does not support video playback.
						</video>
					</div>
				{:else if fileType === 'audio'}
					<div class="flex items-center justify-center bg-gray-50 rounded-lg p-8">
						<div class="w-full max-w-md">
							<div class="mb-4 text-center">
								<svg class="mx-auto h-16 w-16 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
								</svg>
								<p class="mt-2 text-sm font-medium text-gray-700">{file?.original_name}</p>
							</div>
							<!-- svelte-ignore a11y_media_has_caption -->
							<audio src={objectUrl} controls class="w-full">
								Your browser does not support audio playback.
							</audio>
						</div>
					</div>
				{:else if fileType === 'pdf'}
					<div class="h-[70vh] w-full">
						<embed src={objectUrl} type="application/pdf" class="h-full w-full rounded-lg" />
					</div>
				{:else if fileType === 'text'}
					<div class="bg-muted rounded-lg p-4 overflow-auto max-h-[70vh]">
						<pre class="text-sm"><code 
							bind:this={codeElement}
							class="language-{getLanguageFromFileName(file?.original_name || '')}"
						>{textContent}</code></pre>
					</div>
				{:else}
					<div class="flex h-64 items-center justify-center">
						<div class="text-center max-w-md">
							<svg class="mx-auto h-16 w-16 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
							</svg>
							<p class="mt-4 text-sm text-gray-600">Preview not available for this file type</p>
							<p class="mt-2 text-xs text-gray-500">Click download to save the file</p>
						</div>
					</div>
				{/if}
			{/if}
		</div>

		<DialogFooter>
			<Button variant="outline" onclick={handleClose}>Close</Button>
		</DialogFooter>
		</div>
	</DialogContent>
</Dialog>
