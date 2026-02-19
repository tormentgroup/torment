<script lang="ts">
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import type { User } from '$lib/utils/types';

	let { users, pending }: { users: User[]; pending?: boolean } = $props();
	$effect(() => {
		console.log(users)
	})
</script>

{#if pending}
	<Spinner />
{:else}
	<div class="wrapper">
		<p class="users-count">Users &mdash; {users.length}</p>
		{#each users as item}
			<div class="profile-wrapper">
				<div class="profile">
					<img
						src={item.avatar_url ??
							'https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80'}
						width={35}
						height={35}
						class="profile-img"
					/>
					<p class="user-name">{item.display_name}</p>
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	@reference "tailwindcss";
	.user-name {
		font-size: 0.8rem;
		font-weight: 500;
	}
	.users-count {
		font-size: 0.8rem;
	}
	.wrapper {
		display: flex;
		flex-direction: column;
		margin: 0.8rem 0.5rem;
		overflow: auto;
	}

	.profile-wrapper {
		padding: 0.1rem 0;
		width: 100%;
	}

	.profile {
		display: flex;
		align-items: center;
		padding: 0.3rem;
		width: 100%;
		border-radius: 8px;
		gap: 0.5rem;
	}

	.profile-wrapper:hover .profile {
		cursor: pointer;
	}

	.profile-img {
		object-fit: cover;
		object-position: center;
		border-radius: 100%;
		aspect-ratio: 1/1;
	}
</style>
