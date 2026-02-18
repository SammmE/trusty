<script lang="ts">
	import { login, signup, type ApiError } from '$lib/api';
	import { setLoginPassword } from '$lib/session';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';

	interface Props {
		onSuccess: () => void;
	}

	let { onSuccess }: Props = $props();

	let isSignup = $state(false);
	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';

		if (!username || !password) {
			error = 'Please fill in all fields';
			return;
		}

		if (isSignup && password !== confirmPassword) {
			error = 'Passwords do not match';
			return;
		}

		loading = true;

		try {
			if (isSignup) {
				await signup(username, password);
			} else {
				await login(username, password);
			}
			// Store login password in session storage
			setLoginPassword(password);
			onSuccess();
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.body?.error || apiError.message || 'Authentication failed';
		} finally {
			loading = false;
		}
	}

	function toggleMode() {
		isSignup = !isSignup;
		error = '';
		password = '';
		confirmPassword = '';
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-background px-4">
	<div class="w-full max-w-md space-y-8">
		<div class="text-center">
			<h2 class="text-3xl font-bold tracking-tight">
				{isSignup ? 'Create an account' : 'Sign in to Trusty'}
			</h2>
			<p class="mt-2 text-sm text-muted-foreground">
				Secure, encrypted file storage
			</p>
		</div>

		<form class="mt-8 space-y-6" onsubmit={handleSubmit}>
			<div class="space-y-4 rounded-md">
				<div>
					<Label for="username">Username</Label>
					<Input
						id="username"
						type="text"
						bind:value={username}
						placeholder="Enter username"
						required
						disabled={loading}
						class="mt-1"
					/>
				</div>

				<div>
					<Label for="password">Password</Label>
					<Input
						id="password"
						type="password"
						bind:value={password}
						placeholder="Enter password"
						required
						disabled={loading}
						class="mt-1"
					/>
				</div>

				{#if isSignup}
					<div>
						<Label for="confirm-password">Confirm Password</Label>
						<Input
							id="confirm-password"
							type="password"
							bind:value={confirmPassword}
							placeholder="Confirm password"
							required
							disabled={loading}
							class="mt-1"
						/>
					</div>
				{/if}
			</div>

			{#if error}
				<div class="rounded-md bg-red-50 p-4">
					<p class="text-sm text-red-800">{error}</p>
				</div>
			{/if}

			{#if isSignup}
				<div class="rounded-md bg-yellow-50 p-4">
					<p class="text-xs text-yellow-800">
						⚠️ Warning: Your password is used to encrypt your files. If you lose it, your files cannot be recovered.
					</p>
				</div>
			{/if}

			<div class="space-y-3">
				<Button type="submit" class="w-full" disabled={loading}>
					{loading ? 'Please wait...' : isSignup ? 'Sign up' : 'Sign in'}
				</Button>

				<button
					type="button"
					onclick={toggleMode}
					class="w-full text-center text-sm text-muted-foreground hover:text-foreground"
					disabled={loading}
				>
					{isSignup ? 'Already have an account? Sign in' : "Don't have an account? Sign up"}
				</button>
			</div>
		</form>
	</div>
</div>
