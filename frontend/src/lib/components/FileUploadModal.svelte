<script lang="ts">
	import { uploadFile, type ApiError } from '$lib/api';
	import { encryptFile, CryptoError } from '$lib/crypto';
	import { getLoginPassword, getUseLoginPassword } from '$lib/session';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';

	interface Props {
		open: boolean;
		onClose: () => void;
		onSuccess: () => void;
		mode?: 'file' | 'folder';
	}

	let { open = $bindable(), onClose, onSuccess, mode = 'file' }: Props = $props();

	let selectedFile = $state<File | null>(null);
	let selectedFiles = $state<Array<{ file: File; path: string }>>([]);
	let password = $state('');
	let useLoginPasswordForUpload = $state(false);
	let error = $state('');
	let uploading = $state(false);
	let progress = $state(0);
	let currentFileIndex = $state(0);

	// Initialize useLoginPasswordForUpload based on session setting
	$effect(() => {
		if (open) {
			useLoginPasswordForUpload = getUseLoginPassword();
			if (useLoginPasswordForUpload) {
				password = getLoginPassword() || '';
			}
		}
	});

	function handleFileSelect(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files) {
			if (mode === 'folder') {
				const filesArray: Array<{ file: File; path: string }> = [];
				for (let i = 0; i < target.files.length; i++) {
					const file = target.files[i];
					const relativePath = (file as any).webkitRelativePath || file.name;
					filesArray.push({ file, path: relativePath });
				}
				selectedFiles = filesArray;
			} else if (target.files[0]) {
				selectedFile = target.files[0];
			}
			error = '';
		}
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer?.items) {
			const filesArray: Array<{ file: File; path: string }> = [];
			
			for (let i = 0; i < e.dataTransfer.items.length; i++) {
				const item = e.dataTransfer.items[i];
				if (item.kind === 'file') {
					const entry = item.webkitGetAsEntry?.();
					if (entry) {
						await processEntry(entry, '', filesArray);
					} else {
						const file = item.getAsFile();
						if (file) {
							filesArray.push({ file, path: file.name });
						}
					}
				}
			}
			
			if (filesArray.length > 0) {
				if (mode === 'folder') {
					selectedFiles = filesArray;
				} else {
					selectedFile = filesArray[0].file;
				}
				error = '';
			}
		} else if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
			if (mode === 'folder') {
				const filesArray: Array<{ file: File; path: string }> = [];
				for (let i = 0; i < e.dataTransfer.files.length; i++) {
					const file = e.dataTransfer.files[i];
					filesArray.push({ file, path: file.name });
				}
				selectedFiles = filesArray;
			} else {
				selectedFile = e.dataTransfer.files[0];
			}
			error = '';
		}
	}

	async function processEntry(
		entry: any,
		path: string,
		filesArray: Array<{ file: File; path: string }>
	): Promise<void> {
		if (entry.isFile) {
			const file = await new Promise<File>((resolve, reject) => {
				entry.file(resolve, reject);
			});
			const fullPath = path ? `${path}/${file.name}` : file.name;
			filesArray.push({ file, path: fullPath });
		} else if (entry.isDirectory) {
			const dirReader = entry.createReader();
			const entries = await new Promise<any[]>((resolve, reject) => {
				dirReader.readEntries(resolve, reject);
			});
			
			for (const childEntry of entries) {
				const newPath = path ? `${path}/${entry.name}` : entry.name;
				await processEntry(childEntry, newPath, filesArray);
			}
		}
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
	}

	async function handleUpload() {
		// If using login password, ensure we have it
		if (useLoginPasswordForUpload) {
			const loginPwd = getLoginPassword();
			if (loginPwd) {
				password = loginPwd;
			}
		}
		
		if (mode === 'file') {
			if (!selectedFile || !password) {
				error = 'Please select a file and enter a password';
				return;
			}
			await uploadSingleFile(selectedFile, selectedFile.name);
		} else {
			if (selectedFiles.length === 0 || !password) {
				error = 'Please select a folder and enter a password';
				return;
			}
			await uploadMultipleFiles();
		}
	}

	async function uploadSingleFile(file: File, fileName: string) {
		uploading = true;
		error = '';
		progress = 0;

		try {
			progress = 10;
			const { encryptedBlob } = await encryptFile(file, password);

			progress = 50;

			await uploadFile(encryptedBlob, {
				original_name: fileName,
				mime_type: file.type || 'application/octet-stream',
				size_bytes: file.size,
				client_encryption_algo: 'AES-GCM-256',
			});

			progress = 100;
			onSuccess();
			handleClose();
		} catch (err) {
			if (err instanceof CryptoError) {
				error = err.message;
			} else if (err instanceof Error) {
				const apiError = err as ApiError;
				error = apiError.body?.error || apiError.message || 'Upload failed';
			}
		} finally {
			uploading = false;
			progress = 0;
		}
	}

	async function uploadMultipleFiles() {
		uploading = true;
		error = '';
		progress = 0;
		currentFileIndex = 0;

		try {
			const totalFiles = selectedFiles.length;
			
			for (let i = 0; i < totalFiles; i++) {
				currentFileIndex = i;
				const { file, path } = selectedFiles[i];
				
				// Calculate progress for this file
				const fileStartProgress = (i / totalFiles) * 100;
				const fileEndProgress = ((i + 1) / totalFiles) * 100;
				
				progress = fileStartProgress;
				
				const { encryptedBlob } = await encryptFile(file, password);
				
				progress = fileStartProgress + (fileEndProgress - fileStartProgress) * 0.5;

				await uploadFile(encryptedBlob, {
					original_name: path,
					mime_type: file.type || 'application/octet-stream',
					size_bytes: file.size,
					client_encryption_algo: 'AES-GCM-256',
				});

				progress = fileEndProgress;
			}

			onSuccess();
			handleClose();
		} catch (err) {
			if (err instanceof CryptoError) {
				error = err.message;
			} else if (err instanceof Error) {
				const apiError = err as ApiError;
				error = apiError.body?.error || apiError.message || 'Upload failed';
			}
		} finally {
			uploading = false;
			progress = 0;
			currentFileIndex = 0;
		}
	}

	function handleClose() {
		if (!uploading) {
			selectedFile = null;
			selectedFiles = [];
			password = '';
			useLoginPasswordForUpload = false;
			error = '';
			progress = 0;
			currentFileIndex = 0;
			onClose();
		}
	}
</script>

<Dialog bind:open>
	<DialogContent class="sm:max-w-[500px]">
		<DialogHeader>
			<DialogTitle>Upload {mode === 'folder' ? 'Folder' : 'File'}</DialogTitle>
			<DialogDescription>
				{mode === 'folder' 
					? 'Select a folder and enter a password to encrypt all files before upload.' 
					: 'Select a file and enter a password to encrypt it before upload.'}
			</DialogDescription>
		</DialogHeader>

		<div class="space-y-4 py-4">
			{#if (mode === 'file' && !selectedFile) || (mode === 'folder' && selectedFiles.length === 0)}
				<div
					class="flex cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed border-gray-300 p-12 hover:border-gray-400"
					ondrop={handleDrop}
					ondragover={handleDragOver}
					onclick={() => document.getElementById('file-input')?.click()}
					role="button"
					tabindex="0"
				>
					<svg
						class="mb-3 h-10 w-10 text-gray-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
						/>
					</svg>
					<p class="text-sm text-gray-600">
						{mode === 'folder' 
							? 'Click to select or drag and drop a folder' 
							: 'Click to select or drag and drop a file'}
					</p>
					<input
						id="file-input"
						type="file"
						class="hidden"
						onchange={handleFileSelect}
						disabled={uploading}
						{...(mode === 'folder' ? { webkitdirectory: true, directory: true, multiple: true } : {})}
					/>
				</div>
			{:else}
				<div class="rounded-lg bg-gray-50 p-4">
					{#if mode === 'file' && selectedFile}
						<div class="flex items-center justify-between">
							<div class="flex items-center space-x-3">
								<svg
									class="h-8 w-8 text-gray-400"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
									/>
								</svg>
								<div>
									<p class="text-sm font-medium">{selectedFile.name}</p>
									<p class="text-xs text-gray-500">
										{(selectedFile.size / 1024 / 1024).toFixed(2)} MB
									</p>
								</div>
							</div>
							{#if !uploading}
								<button
									type="button"
									onclick={() => (selectedFile = null)}
									class="text-gray-400 hover:text-gray-600"
								>
									<svg class="h-5 w-5" fill="currentColor" viewBox="0 0 20 20">
										<path
											fill-rule="evenodd"
											d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
											clip-rule="evenodd"
										/>
									</svg>
								</button>
							{/if}
						</div>
					{:else if mode === 'folder' && selectedFiles.length > 0}
						<div class="flex items-center justify-between">
							<div class="flex items-center space-x-3">
								<svg
									class="h-8 w-8 text-gray-400"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
									/>
								</svg>
								<div>
									<p class="text-sm font-medium">
										{selectedFiles.length} file{selectedFiles.length !== 1 ? 's' : ''} selected
									</p>
									<p class="text-xs text-gray-500">
										{(selectedFiles.reduce((sum, f) => sum + f.file.size, 0) / 1024 / 1024).toFixed(2)} MB total
									</p>
								</div>
							</div>
							{#if !uploading}
								<button
									type="button"
									onclick={() => (selectedFiles = [])}
									class="text-gray-400 hover:text-gray-600"
								>
									<svg class="h-5 w-5" fill="currentColor" viewBox="0 0 20 20">
										<path
											fill-rule="evenodd"
											d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
											clip-rule="evenodd"
										/>
									</svg>
								</button>
							{/if}
						</div>
						{#if !uploading}
							<div class="mt-3 max-h-40 overflow-y-auto border-t pt-2">
								{#each selectedFiles.slice(0, 10) as { file, path }}
									<p class="truncate text-xs text-gray-600">{path}</p>
								{/each}
								{#if selectedFiles.length > 10}
									<p class="text-xs text-gray-500 italic">...and {selectedFiles.length - 10} more</p>
								{/if}
							</div>
						{/if}
					{/if}
				</div>

				<div>
					<div class="flex items-center justify-between mb-2">
						<Label for="upload-password">Encryption Password</Label>
						<label class="flex items-center gap-2 text-sm cursor-pointer">
							<input
								type="checkbox"
								bind:checked={useLoginPasswordForUpload}
								disabled={uploading}
								class="h-4 w-4 rounded border-gray-300"
								onchange={() => {
									if (useLoginPasswordForUpload) {
										password = getLoginPassword() || '';
									}
								}}
							/>
							<span class="text-gray-600">Use login password</span>
						</label>
					</div>
					<Input
						id="upload-password"
						type="password"
						bind:value={password}
						placeholder="Enter password to encrypt file"
						disabled={uploading || useLoginPasswordForUpload}
						class="mt-1"
					/>
					<p class="mt-1 text-xs text-gray-500">
						Use the same password you use to sign in, or a different one for extra security.
					</p>
				</div>

				{#if uploading}
					<div class="space-y-2">
						<div class="h-2 w-full overflow-hidden rounded-full bg-gray-200">
							<div
								class="h-full bg-blue-600 transition-all duration-300"
								style="width: {progress}%"
							></div>
						</div>
						<p class="text-center text-sm text-gray-600">
							{#if mode === 'folder'}
								Uploading file {currentFileIndex + 1} of {selectedFiles.length}...
							{:else}
								{progress < 50 ? 'Encrypting...' : 'Uploading...'}
							{/if}
						</p>
					</div>
				{/if}

				{#if error}
					<div class="rounded-md bg-red-50 p-3">
						<p class="text-sm text-red-800">{error}</p>
					</div>
				{/if}
			{/if}
		</div>

		<DialogFooter>
			<Button variant="outline" onclick={handleClose} disabled={uploading}>Cancel</Button>
			{#if (mode === 'file' && selectedFile) || (mode === 'folder' && selectedFiles.length > 0)}
				<Button onclick={handleUpload} disabled={uploading || !password}>
					{uploading ? 'Uploading...' : 'Upload'}
				</Button>
			{/if}
		</DialogFooter>
	</DialogContent>
</Dialog>
