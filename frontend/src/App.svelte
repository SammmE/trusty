<script lang="ts">
	import './app.css';
	import { onMount } from 'svelte';
	import { isAuthenticated, logout } from '$lib/api';
	import { clearSession, getUseLoginPassword, setUseLoginPassword } from '$lib/session';
	import { getTheme, setTheme, initTheme, type Theme } from '$lib/theme';
	import FileTable from './lib/components/FileTable.svelte';
	import LoginForm from './lib/components/LoginForm.svelte';
	import FileUploadModal from './lib/components/FileUploadModal.svelte';
	import SystemStats from './lib/components/SystemStats.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Upload, FolderUp, LogOut, Key, KeyRound, Sun, Moon, Monitor, Activity } from 'lucide-svelte';

	let authenticated = $state(false);
	let showUploadModal = $state(false);
	let showFolderUploadModal = $state(false);
	let showSystemStats = $state(false);
	let refreshKey = $state(0);
	let useLoginPassword = $state(false);
	let currentTheme = $state<Theme>('system');

	function checkAuth() {
		authenticated = isAuthenticated();
		if (authenticated) {
			useLoginPassword = getUseLoginPassword();
		}
	}

	function handleLoginSuccess() {
		checkAuth();
	}

	function handleLogout() {
		logout();
		clearSession();
		checkAuth();
	}

	function toggleUseLoginPassword() {
		useLoginPassword = !useLoginPassword;
		setUseLoginPassword(useLoginPassword);
	}

	function cycleTheme() {
		const themes: Theme[] = ['light', 'dark', 'system'];
		const currentIndex = themes.indexOf(currentTheme);
		const nextTheme = themes[(currentIndex + 1) % themes.length];
		currentTheme = nextTheme;
		setTheme(nextTheme);
	}

	function getThemeIcon() {
		if (currentTheme === 'light') return Sun;
		if (currentTheme === 'dark') return Moon;
		return Monitor;
	}

	function handleUploadSuccess() {
		refreshKey++;
	}

	onMount(() => {
		currentTheme = getTheme();
		checkAuth();
		
		// Clear session data when window/tab closes
		const handleBeforeUnload = () => {
			// Session storage will auto-clear, but ensure everything is cleaned up
			clearSession();
		};
		
		window.addEventListener('beforeunload', handleBeforeUnload);
		
		return () => {
			window.removeEventListener('beforeunload', handleBeforeUnload);
		};
	});
</script>

{#if !authenticated}
	<LoginForm onSuccess={handleLoginSuccess} />
{:else}
	<div class="flex h-screen flex-col bg-background">
		<header class="border-b bg-card">
			<div class="mx-auto flex items-center justify-between px-6 py-3">
				<h1 class="text-2xl font-bold">Trusty</h1>
				<div class="flex items-center gap-2">
					<Button 
						variant="outline"
						size="icon"
						onclick={cycleTheme}
						title={`Theme: ${currentTheme}`}
					>
						{#if currentTheme === 'light'}
							<Sun class="h-4 w-4" />
						{:else if currentTheme === 'dark'}
							<Moon class="h-4 w-4" />
						{:else}
							<Monitor class="h-4 w-4" />
						{/if}
					</Button>
					<Button 
						variant="outline"
						onclick={() => (showSystemStats = true)}
						title="System Statistics"
					>
						<Activity class="mr-2 h-4 w-4" />
						Stats
					</Button>
					<Button 
						variant={useLoginPassword ? 'default' : 'outline'} 
						onclick={toggleUseLoginPassword}
						title={useLoginPassword ? 'Using login password for encryption' : 'Click to use login password'}
					>
						{#if useLoginPassword}
							<Key class="mr-2 h-4 w-4" />
							Using Login Password
						{:else}
							<KeyRound class="mr-2 h-4 w-4" />
							Use Login Password
						{/if}
					</Button>
					<Button onclick={() => (showUploadModal = true)}>
						<Upload class="mr-2 h-4 w-4" />
						Upload File
					</Button>
					<Button variant="outline" onclick={() => (showFolderUploadModal = true)}>
						<FolderUp class="mr-2 h-4 w-4" />
						Upload Folder
					</Button>
					<Button variant="outline" onclick={handleLogout}>
						<LogOut class="mr-2 h-4 w-4" />
						Logout
					</Button>
				</div>
			</div>
		</header>

		<main class="flex-1 overflow-hidden px-6 py-4">
			<div class="h-full">
				<FileTable refresh={refreshKey} />
			</div>
		</main>
	</div>

	<FileUploadModal
		bind:open={showUploadModal}
		onClose={() => (showUploadModal = false)}
		onSuccess={handleUploadSuccess}
		mode="file"
	/>
	
	<FileUploadModal
		bind:open={showFolderUploadModal}
		onClose={() => (showFolderUploadModal = false)}
		onSuccess={handleUploadSuccess}
		mode="folder"
	/>

	<SystemStats
		bind:open={showSystemStats}
		onClose={() => (showSystemStats = false)}
	/>
{/if}

