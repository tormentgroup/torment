<script lang="ts">
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import type { Message, User } from '../../lib/utils/types';
	import { formatDate } from '../../lib/utils/utils';
	let { messages, users, pending }: { messages: Message[]; users: User[]; pending?: boolean } =
		$props();
</script>

<div class="wrapper">
	{#if pending}
		<Spinner />
	{:else}
		{#each messages as item}
			<div class="message">
				<img src={users.find((u) => u.id == item.sender)?.avatar_url} class="profile" alt="fuck" />
				<div class="inner-wrapper">
					<div class="title">
						<p class="name">{users.find((u) => u.id == item.sender)?.display_name}</p>
						{formatDate(new Date(item.ts_ms))}
					</div>
					{#if item.formatted_html}
						<p>{@html item.formatted_html}</p>
					{:else}
						<p>{item.body}</p>
					{/if}
				</div>
			</div>
		{/each}
	{/if}
</div>

<style>
	.title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.name {
		font-weight: 500;
		margin-top: 0;
	}

	.profile {
		object-fit: cover;
		object-position: center;
		width: 2.7rem;
		height: 2.7rem;
		aspect-ratio: 1/1;
		border-radius: 100%;
	}

	.wrapper {
		display: flex;
		flex-direction: column;
        margin-bottom: 1rem;
		gap: 2rem;
		overflow-y: auto;
		justify-content: flex-end;
		height: 100%;
	}

	.message {
		display: flex;
		font-weight: 300;
		gap: 0.5rem;
	}
</style>
