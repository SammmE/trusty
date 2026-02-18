<script lang="ts">
	import { onMount } from 'svelte';
	import { listFiles, downloadFile, deleteFile, uploadFile, type FileMetadata, type FileListResponse, type ApiError } from '$lib/api';
	import { decryptFile, downloadDecryptedFile, encryptFile, CryptoError } from '$lib/crypto';
	import { getLoginPassword, getUseLoginPassword } from '$lib/session';
	import FilePreview from '$lib/components/FilePreview.svelte';
	import {
		Table,
		TableBody,
		TableCell,
		TableHead,
		TableHeader,
		TableRow
	} from '$lib/components/ui/table';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';
	import * as ContextMenu from '$lib/components/ui/context-menu';
	import { Button } from '$lib/components/ui/button';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import { Label } from '$lib/components/ui/label';
	import { 
		File as FileIcon, 
		Folder, 
		Home, 
		ChevronRight, 
		MoreHorizontal, 
		ArrowUpDown, 
		Search,
		FileText,
		FileCode,
		FileJson,
		FileImage,
		FileVideo,
		FileAudio,
		FileArchive,
		FileSpreadsheet,
		Presentation,
		FileType,
		Download,
		Trash2
	} from 'lucide-svelte';

	interface Props {
		refresh?: number;
	}

	let { refresh = 0 }: Props = $props();

	let allFiles: FileMetadata[] = $state([]);
	let currentPath = $state('');
	let loading = $state(true);
	let searchQuery = $state('');
	let searchDebounceTimer: any;
	let sortColumn = $state('created_at');
	let sortDirection: 'asc' | 'desc' = $state('desc');
	let error = $state('');

	// Pagination state
	let currentPage = $state(1);
	let pageSize = $state(20);
	let totalFiles = $state(0);
	let totalPages = $state(0);
	let allPagesLoaded = $state(false);

	let downloadingId = $state<string | null>(null);
	let showPasswordDialog = $state(false);
	let passwordDialogFile = $state<FileMetadata | null>(null);
	let downloadPassword = $state('');
	let passwordError = $state('');

	let deletingId = $state<string | null>(null);
	let contextMenuFile = $state<FileMetadata | null>(null);
	let selectedItem = $state<string | null>(null);
	let selectedFile = $state<FileMetadata | null>(null);
	
	// Preview state
	let showPreview = $state(false);
	let previewFile = $state<FileMetadata | null>(null);
	
	// New folder dialog
	let showNewFolderDialog = $state(false);
	let newFolderName = $state('');
	let newFolderPassword = $state('');
	let creatingFolder = $state(false);

	// Local search results (shown immediately before server response)
	let localSearchResults = $state<FileMetadata[]>([]);
	let showLocalResults = $state(false);

	interface DirectoryItem {
		name: string;
		fullPath: string;
		isFolder: boolean;
		file?: FileMetadata;
	}

	// Get items in current directory
	let currentItems = $derived.by(() => {
		const items: DirectoryItem[] = [];
		const folders = new Set<string>();

		const filesToProcess = showLocalResults ? localSearchResults : allFiles;

		filesToProcess.forEach(file => {
			const relativePath = currentPath 
				? file.original_name.startsWith(currentPath + '/')
					? file.original_name.substring(currentPath.length + 1)
					: null
				: file.original_name;

			if (!relativePath) return;

			const parts = relativePath.split('/');
			
			if (parts.length === 1) {
				// File in current directory
				items.push({
					name: parts[0],
					fullPath: file.original_name,
					isFolder: false,
					file
				});
			} else {
				// Folder in current directory
				const folderName = parts[0];
				const folderPath = currentPath ? `${currentPath}/${folderName}` : folderName;
				
				if (!folders.has(folderPath)) {
					folders.add(folderPath);
					items.push({
						name: folderName,
						fullPath: folderPath,
						isFolder: true
					});
				}
			}
		});

		// Sort: folders first, then by name
		items.sort((a, b) => {
			if (a.isFolder && !b.isFolder) return -1;
			if (!a.isFolder && b.isFolder) return 1;
			return a.name.localeCompare(b.name);
		});

		return items;
	});

	// Breadcrumb navigation
	let breadcrumbs = $derived.by(() => {
		if (!currentPath) return [];
		const parts = currentPath.split('/');
		const crumbs = [];
		let path = '';
		for (const part of parts) {
			path = path ? `${path}/${part}` : part;
			crumbs.push({ name: part, path });
		}
		return crumbs;
	});

	async function loadData() {
		loading = true;
		error = '';
		showLocalResults = false;
		
		try {
			const sortField = sortColumn === 'created_at' ? 'date' : sortColumn;
			const response = await listFiles(
				searchQuery || undefined, 
				sortField, 
				sortDirection,
				currentPage,
				pageSize
			);
			
			allFiles = response.files;
			totalFiles = response.total;
			totalPages = response.total_pages;
			currentPage = response.page;
			
			// Check if all pages are loaded by loading all pages
			if (currentPage === 1 && totalPages > 1) {
				allPagesLoaded = false;
			} else if (currentPage >= totalPages) {
				allPagesLoaded = true;
			}
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.body?.error || apiError.message || 'Failed to load files';
		} finally {
			loading = false;
		}
	}

	async function loadAllPages() {
		// Load all pages for local search
		if (allPagesLoaded) return;
		
		try {
			const sortField = sortColumn === 'created_at' ? 'date' : sortColumn;
			const allPagesData: FileMetadata[] = [];
			
			for (let page = 1; page <= totalPages; page++) {
				const response = await listFiles(
					searchQuery || undefined,
					sortField,
					sortDirection,
					page,
					pageSize
				);
				allPagesData.push(...response.files);
			}
			
			allFiles = allPagesData;
			allPagesLoaded = true;
		} catch (err) {
			console.error('Failed to load all pages:', err);
		}
	}

	function toggleSort(col: string) {
		if (sortColumn === col) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			sortColumn = col;
			sortDirection = 'asc';
		}
		currentPage = 1;
		loadData();
	}

	function handleSearch(e: Event) {
		const query = (e.target as HTMLInputElement).value;
		searchQuery = query;
		
		// Show local results immediately if all pages are loaded
		if (allPagesLoaded && query) {
			showLocalResults = true;
			localSearchResults = allFiles.filter(f => 
				f.original_name.toLowerCase().includes(query.toLowerCase())
			);
		}
		
		// Debounce server search
		clearTimeout(searchDebounceTimer);
		searchDebounceTimer = setTimeout(() => {
			currentPage = 1;
			loadData();
		}, 300);
	}

	function navigateToFolder(path: string) {
		currentPath = path;
	}

	function navigateUp() {
		if (!currentPath) return;
		const parts = currentPath.split('/');
		parts.pop();
		currentPath = parts.join('/');
	}

	function navigateToRoot() {
		currentPath = '';
	}

	function handleItemClick(item: DirectoryItem) {
		if (item.isFolder) {
			selectedItem = null;
			selectedFile = null;
		} else {
			selectedItem = item.file?.id || null;
			selectedFile = item.file || null;
		}
	}

	function handleItemDoubleClick(item: DirectoryItem) {
		if (item.isFolder) {
			navigateToFolder(item.fullPath);
			selectedItem = null;
			selectedFile = null;
		} else if (item.file) {
			// Preview the file
			previewFile = item.file;
			showPreview = true;
		}
	}

	function getFileIcon(fileName: string) {
		const ext = fileName.toLowerCase().split('.').pop();
		
		// Code files
		if (['js', 'ts', 'jsx', 'tsx', 'py', 'java', 'cpp', 'c', 'h', 'hpp', 'cs', 'php', 'rb', 'go', 'rs', 'swift', 'kt'].includes(ext || '')) {
			return FileCode;
		}
		
		// JSON/Config files
		if (['json', 'xml', 'yaml', 'yml', 'toml', 'ini', 'config'].includes(ext || '')) {
			return FileJson;
		}
		
		// Image files
		if (['jpg', 'jpeg', 'png', 'gif', 'bmp', 'svg', 'webp', 'ico'].includes(ext || '')) {
			return FileImage;
		}
		
		// Video files
		if (['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'm4v'].includes(ext || '')) {
			return FileVideo;
		}
		
		// Audio files
		if (['mp3', 'wav', 'ogg', 'flac', 'm4a', 'aac', 'wma'].includes(ext || '')) {
			return FileAudio;
		}
		
		// Archive files
		if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'tgz'].includes(ext || '')) {
			return FileArchive;
		}
		
		// Spreadsheet files
		if (['xls', 'xlsx', 'csv', 'ods'].includes(ext || '')) {
			return FileSpreadsheet;
		}
		
		// Presentation files
		if (['ppt', 'pptx', 'odp', 'key'].includes(ext || '')) {
			return Presentation;
		}
		
		// PDF files
		if (ext === 'pdf') {
			return FileType;
		}
		
		// Text files
		if (['txt', 'md', 'markdown', 'log', 'rtf'].includes(ext || '')) {
			return FileText;
		}
		
		// Default
		return FileIcon;
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + ' KB';
		if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
		return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
	}

	function formatDate(dateString: string): string {
		const date = new Date(dateString);
		return date.toLocaleDateString() + ' ' + date.toLocaleTimeString();
	}

	function promptDownload(file: FileMetadata) {
		// First try with login password if enabled
		const useLoginPwd = getUseLoginPassword();
		const loginPwd = getLoginPassword();
		
		if (useLoginPwd && loginPwd) {
			tryDownloadWithPassword(file, loginPwd);
			return;
		}
		
		// Otherwise show password dialog
		passwordDialogFile = file;
		downloadPassword = '';
		passwordError = '';
		showPasswordDialog = true;
	}

	async function tryDownloadWithPassword(file: FileMetadata, pwd: string) {
		downloadingId = file.id;
		try {
			const encryptedBlob = await downloadFile(file.id);
			const decryptedBlob = await decryptFile(encryptedBlob, pwd, file.original_name);
			downloadDecryptedFile(decryptedBlob, file.original_name);
		} catch (err) {
			// If login password failed, show password dialog
			if (err instanceof CryptoError) {
				passwordDialogFile = file;
				downloadPassword = '';
				passwordError = 'Login password failed. Please enter the correct password.';
				showPasswordDialog = true;
			} else if (err instanceof Error) {
				const apiError = err as ApiError;
				alert(apiError.body?.error || apiError.message || 'Download failed');
			}
		} finally {
			downloadingId = null;
		}
	}

	async function handleDownload() {
		if (!passwordDialogFile || !downloadPassword) {
			passwordError = 'Please enter a password';
			return;
		}

		const file = passwordDialogFile;
		downloadingId = file.id;
		passwordError = '';

		try {
			const encryptedBlob = await downloadFile(file.id);
			const decryptedBlob = await decryptFile(encryptedBlob, downloadPassword, file.original_name);
			downloadDecryptedFile(decryptedBlob, file.original_name);

			showPasswordDialog = false;
			passwordDialogFile = null;
			downloadPassword = '';
		} catch (err) {
			if (err instanceof CryptoError) {
				passwordError = 'Decryption failed - wrong password';
			} else if (err instanceof Error) {
				const apiError = err as ApiError;
				passwordError = apiError.body?.error || apiError.message || 'Download failed';
			}
		} finally {
			downloadingId = null;
		}
	}

	async function handleDelete(file: FileMetadata) {
		if (!confirm(`Are you sure you want to delete "${file.original_name}"?`)) {
			return;
		}

		deletingId = file.id;
		try {
			await deleteFile(file.id);
			await loadData();
		} catch (err) {
			const apiError = err as ApiError;
			alert(apiError.body?.error || apiError.message || 'Delete failed');
		} finally {
			deletingId = null;
		}
	}

	function showNewFolderPrompt() {
		newFolderName = '';
		newFolderPassword = '';
		showNewFolderDialog = true;
	}

	async function handleCreateFolder() {
		if (!newFolderName || !newFolderPassword) {
			return;
		}

		creatingFolder = true;
		try {
			// Create a placeholder file to represent the folder
			const folderPath = currentPath ? `${currentPath}/${newFolderName}` : newFolderName;
			const placeholderName = `${folderPath}/.folder`;
			
			// Create an empty blob for the placeholder
			const text = new Blob([''], { type: 'text/plain' });
			const emptyFile = new File([text], '.folder', { type: 'text/plain' });
			const { encryptedBlob } = await encryptFile(emptyFile, newFolderPassword);

			await uploadFile(encryptedBlob, {
				original_name: placeholderName,
				mime_type: 'text/plain',
				size_bytes: 0,
				client_encryption_algo: 'AES-GCM-256',
			});

			showNewFolderDialog = false;
			await loadData();
		} catch (err) {
			alert('Failed to create folder');
		} finally {
			creatingFolder = false;
		}
	}

	function goToPage(page: number) {
		if (page < 1 || page > totalPages) return;
		currentPage = page;
		loadData();
	}

	// Generate page numbers to display
	let pageNumbers = $derived.by(() => {
		const pages = [];
		const maxVisible = 7;
		
		if (totalPages <= maxVisible) {
			for (let i = 1; i <= totalPages; i++) {
				pages.push(i);
			}
		} else {
			if (currentPage <= 4) {
				for (let i = 1; i <= 5; i++) pages.push(i);
				pages.push(-1); // ellipsis
				pages.push(totalPages);
			} else if (currentPage >= totalPages - 3) {
				pages.push(1);
				pages.push(-1);
				for (let i = totalPages - 4; i <= totalPages; i++) pages.push(i);
			} else {
				pages.push(1);
				pages.push(-1);
				for (let i = currentPage - 1; i <= currentPage + 1; i++) pages.push(i);
				pages.push(-1);
				pages.push(totalPages);
			}
		}
		
		return pages;
	});

	$effect(() => {
		if (refresh !== undefined) {
			currentPage = 1;
			selectedItem = null;
			selectedFile = null;
			loadData();
		}
	});

	// Clear selection when navigating
	$effect(() => {
		if (currentPath !== undefined) {
			selectedItem = null;
			selectedFile = null;
		}
	});

	onMount(() => {
		loadData();
	});
</script>

<div class="flex h-full flex-col space-y-4">
	{#if error}
		<div class="rounded-md bg-red-50 p-4">
			<p class="text-sm text-red-800">{error}</p>
		</div>
	{/if}

	<!-- Search and breadcrumbs -->
	<div class="flex items-center justify-between gap-4">
		<div class="flex items-center gap-2 flex-1">
			<!-- Breadcrumb navigation -->
			<Button variant="ghost" size="icon" onclick={navigateToRoot}>
				<Home class="h-4 w-4" />
			</Button>
			
			{#each breadcrumbs as crumb, i}
				<ChevronRight class="h-4 w-4 text-muted-foreground" />
				<Button 
					variant="ghost" 
					class="h-8"
					onclick={() => navigateToFolder(crumb.path)}
				>
					{crumb.name}
				</Button>
			{/each}
		</div>
		
		<div class="flex items-center gap-4">
			<div class="relative w-64">
				<Search class="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
				<Input 
					placeholder="Search files..." 
					class="pl-8" 
					oninput={handleSearch}
					value={searchQuery}
				/>
			</div>
			<div class="text-sm text-muted-foreground">
				{totalFiles} item{totalFiles !== 1 ? 's' : ''}
			</div>
		</div>
	</div>

	<!-- File table -->
	<div class="flex-1 overflow-auto rounded-md border shadow-sm">
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead class="w-[50px]"></TableHead>

					<TableHead>
						<Button variant="ghost" onclick={() => toggleSort('original_name')}>
							Name <ArrowUpDown class="ml-2 h-4 w-4" />
						</Button>
					</TableHead>

					<TableHead>
						<Button variant="ghost" onclick={() => toggleSort('size_bytes')}>
							Size <ArrowUpDown class="ml-2 h-4 w-4" />
						</Button>
					</TableHead>

					<TableHead>
						<Button variant="ghost" onclick={() => toggleSort('created_at')}>
							Date Modified <ArrowUpDown class="ml-2 h-4 w-4" />
						</Button>
					</TableHead>

					<TableHead class="text-right">Actions</TableHead>
				</TableRow>
			</TableHeader>

			<TableBody>
				{#if loading}
					{#each Array(10) as _}
						<TableRow>
							<TableCell><Skeleton class="h-8 w-8 rounded" /></TableCell>
							<TableCell><Skeleton class="h-4 w-[200px]" /></TableCell>
							<TableCell><Skeleton class="h-4 w-[80px]" /></TableCell>
							<TableCell><Skeleton class="h-4 w-[100px]" /></TableCell>
							<TableCell><Skeleton class="ml-auto h-8 w-8 rounded-full" /></TableCell>
						</TableRow>
					{/each}
				{:else if currentItems.length === 0}
					<TableRow>
						<TableCell colspan={5} class="h-24 text-center"> 
							{searchQuery ? 'No files found.' : 'This folder is empty.'} 
						</TableCell>
					</TableRow>
				{:else}
					{#each currentItems as item}
						<ContextMenu.Root>
							<ContextMenu.Trigger
								class="contents"
								oncontextmenu={() => (contextMenuFile = item.file || null)}
							>
								<TableRow 
								class="cursor-pointer hover:bg-muted/50 {selectedItem === item.file?.id ? 'bg-primary/10 border-l-4 border-primary' : ''}"
									onclick={() => handleItemClick(item)}
									ondblclick={() => handleItemDoubleClick(item)}
								>
									<TableCell>
										<div class="flex h-8 w-8 items-center justify-center rounded bg-muted">
											{#if item.isFolder}
												<Folder class="h-4 w-4 opacity-70" />
											{:else}
												{#if getFileIcon(item.name) === FileCode}
													<FileCode class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileJson}
													<FileJson class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileImage}
													<FileImage class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileVideo}
													<FileVideo class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileAudio}
													<FileAudio class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileArchive}
													<FileArchive class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileSpreadsheet}
													<FileSpreadsheet class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === Presentation}
													<Presentation class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileType}
													<FileType class="h-4 w-4 opacity-70" />
												{:else if getFileIcon(item.name) === FileText}
													<FileText class="h-4 w-4 opacity-70" />
												{:else}
													<FileIcon class="h-4 w-4 opacity-70" />
												{/if}
											{/if}
										</div>
									</TableCell>
									<TableCell class="font-medium">{item.name}</TableCell>
									<TableCell class="text-muted-foreground">
										{item.isFolder ? '—' : formatFileSize(item.file!.size_bytes)}
									</TableCell>
									<TableCell class="text-muted-foreground">
										{item.isFolder ? '—' : formatDate(item.file!.created_at)}
									</TableCell>
									<TableCell class="text-right">
										{#if !item.isFolder && item.file}
											<DropdownMenu>
												<DropdownMenuTrigger>
													<Button variant="ghost" size="icon">
														<MoreHorizontal class="h-4 w-4" />
													</Button>
												</DropdownMenuTrigger>
												<DropdownMenuContent align="end">
													<DropdownMenuItem
														onclick={() => item.file && promptDownload(item.file)}
														disabled={downloadingId === item.file.id}
													>
														{downloadingId === item.file.id ? 'Downloading...' : 'Download'}
													</DropdownMenuItem>
													<DropdownMenuItem
														class="text-red-600"
														onclick={() => item.file && handleDelete(item.file)}
														disabled={deletingId === item.file.id}
													>
														{deletingId === item.file.id ? 'Deleting...' : 'Delete'}
													</DropdownMenuItem>
												</DropdownMenuContent>
											</DropdownMenu>
										{/if}
									</TableCell>
								</TableRow>
							</ContextMenu.Trigger>

							<ContextMenu.Content>
								<ContextMenu.Item onclick={showNewFolderPrompt}>
									<Folder class="mr-2 h-4 w-4" />
									New Folder
								</ContextMenu.Item>
								{#if item.file}
									<ContextMenu.Separator />
									<ContextMenu.Item onclick={() => contextMenuFile && promptDownload(contextMenuFile)}>
										Download File
									</ContextMenu.Item>
									<ContextMenu.Separator />
									<ContextMenu.Item
										variant="destructive"
										onclick={() => contextMenuFile && handleDelete(contextMenuFile)}
									>
										Delete
									</ContextMenu.Item>
								{/if}
							</ContextMenu.Content>
						</ContextMenu.Root>
					{/each}
				{/if}
			</TableBody>
		</Table>
	</div>

	<!-- Pagination controls -->
	{#if totalPages > 1}
		<div class="flex items-center justify-between px-2">
			<div class="text-sm text-muted-foreground">
				Page {currentPage} of {totalPages}
			</div>
			
			<div class="flex items-center gap-1">
				<Button
					variant="outline"
					size="sm"
					onclick={() => goToPage(currentPage - 1)}
					disabled={currentPage === 1}
				>
					Previous
				</Button>
				
				{#each pageNumbers as page}
					{#if page === -1}
						<span class="px-2">...</span>
					{:else}
						<Button
							variant={page === currentPage ? 'default' : 'outline'}
							size="sm"
							class="w-10"
							onclick={() => goToPage(page)}
						>
							{page}
						</Button>
					{/if}
				{/each}
				
				<Button
					variant="outline"
					size="sm"
					onclick={() => goToPage(currentPage + 1)}
					disabled={currentPage === totalPages}
				>
					Next
				</Button>
			</div>
			
			{#if !allPagesLoaded}
				<Button variant="ghost" size="sm" onclick={loadAllPages}>
					Load All for Search
				</Button>
			{/if}
		</div>
	{/if}

	<!-- Action bar (notch) for selected file -->
	{#if selectedFile}
		<div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-50">
			<div class="flex items-center gap-2 bg-background border-2 border-primary rounded-full shadow-lg px-6 py-3">
				<Button
					variant="ghost"
					size="icon"
					class="h-10 w-10 rounded-full"
					onclick={() => selectedFile && promptDownload(selectedFile)}
					disabled={downloadingId === selectedFile.id}
					title="Download"
				>
					<Download class="h-5 w-5" />
				</Button>
				<div class="h-6 w-px bg-border"></div>
				<Button
					variant="ghost"
					size="icon"
					class="h-10 w-10 rounded-full hover:bg-red-100 hover:text-red-600"
					onclick={() => selectedFile && handleDelete(selectedFile)}
					disabled={deletingId === selectedFile.id}
					title="Delete"
				>
					<Trash2 class="h-5 w-5" />
				</Button>
			</div>
		</div>
	{/if}
</div>

<!-- Download password dialog -->
<Dialog bind:open={showPasswordDialog}>
	<DialogContent class="sm:max-w-[425px]">
		<DialogHeader>
			<DialogTitle>Download File</DialogTitle>
			<DialogDescription>
				Enter the password to decrypt "{passwordDialogFile?.original_name}"
			</DialogDescription>
		</DialogHeader>

		<div class="space-y-4 py-4">
			<div>
				<Label for="download-password">Decryption Password</Label>
				<Input
					id="download-password"
					type="password"
					bind:value={downloadPassword}
					placeholder="Enter password"
					disabled={downloadingId !== null}
					class="mt-1"
					onkeydown={(e) => e.key === 'Enter' && handleDownload()}
				/>
			</div>

			{#if passwordError}
				<div class="rounded-md bg-red-50 p-3">
					<p class="text-sm text-red-800">{passwordError}</p>
				</div>
			{/if}
		</div>

		<DialogFooter>
			<Button
				variant="outline"
				onclick={() => (showPasswordDialog = false)}
				disabled={downloadingId !== null}
			>
				Cancel
			</Button>
			<Button onclick={handleDownload} disabled={downloadingId !== null || !downloadPassword}>
				{downloadingId ? 'Downloading...' : 'Download'}
			</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

<!-- New folder dialog -->
<Dialog bind:open={showNewFolderDialog}>
	<DialogContent class="sm:max-w-[425px]">
		<DialogHeader>
			<DialogTitle>Create New Folder</DialogTitle>
			<DialogDescription>
				Enter a name for the new folder and a password to protect it.
			</DialogDescription>
		</DialogHeader>

		<div class="space-y-4 py-4">
			<div>
				<Label for="folder-name">Folder Name</Label>
				<Input
					id="folder-name"
					type="text"
					bind:value={newFolderName}
					placeholder="New Folder"
					disabled={creatingFolder}
					class="mt-1"
				/>
			</div>
			
			<div>
				<Label for="folder-password">Password</Label>
				<Input
					id="folder-password"
					type="password"
					bind:value={newFolderPassword}
					placeholder="Enter password"
					disabled={creatingFolder}
					class="mt-1"
					onkeydown={(e) => e.key === 'Enter' && handleCreateFolder()}
				/>
			</div>
		</div>

		<DialogFooter>
			<Button
				variant="outline"
				onclick={() => (showNewFolderDialog = false)}
				disabled={creatingFolder}
			>
				Cancel
			</Button>
			<Button 
				onclick={handleCreateFolder} 
				disabled={creatingFolder || !newFolderName || !newFolderPassword}
			>
				{creatingFolder ? 'Creating...' : 'Create Folder'}
			</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

<!-- File Preview Dialog -->
<FilePreview 
	bind:open={showPreview} 
	file={previewFile}
	onClose={() => {
		showPreview = false;
		previewFile = null;
	}}
/>
