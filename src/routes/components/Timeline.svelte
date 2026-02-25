<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import type { Message, User } from '../../lib/utils/types';
	import { formatDate } from '../../lib/utils/utils';
	import { listen } from '@tauri-apps/api/event';
	import { tick } from 'svelte';
	let {
		users,
		roomId
	}: {
		users: User[];
		roomId: string;
	} = $props();

	let scroller: HTMLDivElement;

	let timelinePending = $state(true);
	let loadTopPending = $state(false);
	let anchor: null | { prevHeight: number; prevTop: number; token: number } = null;
	let anchorToken = 0;
	let scrollLoading = false;
	let messages: Message[] = $state([]);
	let taskid = 0;

	$effect(() => {
		(async () => {
			let id = ++taskid;
			messages = [];
			timelinePending = true;
			let mAsync = invoke('open_room', { room_id: roomId });
			mAsync
				.then(async (m) => {
					if (taskid != id) {
						return;
					}
					messages = m as Message[];
					console.log(m);
					await tick();
					requestAnimationFrame(() => {
						if (!scroller) return;
						scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight;
					});
				})
				.finally(() => {
					if (taskid != id) {
						return;
					}
					timelinePending = false;
				});
		})();
	});

	$effect(() => {
		const unlisten = listen('timeline-patch', (event: any) => {
			if (event.payload.room_id != roomId) {
				return;
			}
			let next = messages;
			for (let patch of event.payload.patches) {
				console.log('PATCH: ', patch);
				switch (patch.op) {
					case 'set':
						next = [...next.slice(0, patch.index), patch.row, ...next.slice(patch.index + 1)];
						break;
					case 'push_back':
						next = [...next, patch.row];
						break;
					case 'push_front':
						next = [patch.row, ...next];
						break;
					case 'remove':
						next = [...next.slice(0, patch.index), ...next.slice(patch.index + 1)];
						break;
					case 'insert':
						next = [...next.slice(0, patch.index), patch.row, ...next.slice(patch.index)];
						break;
					default:
						console.error('UNHANDLED TIMELINE EVENT: ', patch);
						break;
				}
			}
			messages = next;
		});
		return () => {
			unlisten.then((v) => v());
		};
	});

	const handleLoadTop = async () => {
		if (!scroller || loadTopPending) return;

		anchor = {
			prevHeight: scroller.scrollHeight,
			prevTop: scroller.scrollTop,
			token: ++anchorToken
		};

		loadTopPending = true;
		try {
			await invoke('paginate_up');
		} finally {
			loadTopPending = false;
		}
	};

	$effect(() => {
		messages.length;
		if (!scroller || !anchor) return;

		const { prevHeight, prevTop, token } = anchor;

		(async () => {
			await tick();

			if (!anchor || anchor.token !== token) return;

			const newHeight = scroller.scrollHeight;
			scroller.scrollTop = prevTop + (newHeight - prevHeight);

			anchor = null;
		})();
	});

	const onScroll = async () => {
		if (!scroller) return;
		if (scroller.scrollTop <= 100 && !loadTopPending && !scrollLoading) {
			scrollLoading = true;
			await handleLoadTop();
			scrollLoading = false;
		}
	};
</script>

<div class="wrapper" bind:this={scroller} onscroll={onScroll}>
	{#if timelinePending}
		<Spinner />
	{:else}
		<div class="mx-auto pb-[5rem]">
			<Button onclick={handleLoadTop}
				>Load More
				{#if loadTopPending}
					<Spinner />
				{/if}
			</Button>
		</div>
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
		height: 100%;
	}

	.message {
		display: flex;
		font-weight: 300;
		gap: 0.5rem;
	}
</style>
