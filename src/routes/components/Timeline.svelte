<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import type { Message, User } from '../../lib/utils/types';
	import { formatDate } from '../../lib/utils/utils';
	import { listen } from '@tauri-apps/api/event';
	import { flushSync, tick } from 'svelte';
	let {
		users,
		roomId
	}: {
		users: User[];
		roomId: string;
	} = $props();

	let scroller: HTMLDivElement;
	type ViewportAnchor = {
		key: string;
		topInViewport: number;
	};

	const getTopIntersectingMessageAnchor = (
		scroller: HTMLElement,
		messageSelector = '.message'
	): ViewportAnchor | null => {
		const sr = scroller.getBoundingClientRect();
		const nodes = scroller.querySelectorAll<HTMLElement>(messageSelector);

		let best: { el: HTMLElement; top: number } | null = null;

		for (const el of nodes) {
			const r = el.getBoundingClientRect();
			const intersects = r.bottom > sr.top && r.top < sr.bottom;
			if (!intersects) continue;
			if (!best || r.top < best.top) {
				best = { el, top: r.top };
			}
		}

		if (!best) return null;

		const key = best.el.dataset.key;
		if (!key) return null;

		return {
			key,
			topInViewport: best.el.getBoundingClientRect().top - sr.top
		};
	};
	const restoreToMessageAnchor = (
		scroller: HTMLElement,
		anchor: ViewportAnchor | null,
		messageSelector = '.message'
	) => {
		if (!anchor) return;

		const sr = scroller.getBoundingClientRect();
		const el = scroller.querySelector<HTMLElement>(
			`${messageSelector}[data-key="${CSS.escape(anchor.key)}"]`
		);
		if (!el) return;

		const currentTopInViewport = el.getBoundingClientRect().top - sr.top;
		const delta = currentTopInViewport - anchor.topInViewport;
		scroller.scrollTop += delta;
	};

	let timelinePending = $state(true);
	let loadTopPending = $state(false);
	let scrollLoading = false;
	let messages: Message[] = $state([]);
	let frozenDeltaY = 0;
	let anchor: ViewportAnchor | null = null;
	let taskid = 0;
	let pendingAnchor = $state(false);

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
					messages = (m as any[]).map((i) => {
						if (i.kind == 'message') {
							return {
								...i.message,
								key: i.key
							};
						} else {
							return {
								key: i.key
							};
						}
					});

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
				let patch_message: Message;
				if (patch.row.kind == 'message') {
					patch_message = { ...patch.row.message, key: patch.row.key };
				} else {
					patch_message = { ...({} as any), key: patch.row.key };
				}
				switch (patch.op) {
					case 'set':
						next = [...next.slice(0, patch.index), patch_message, ...next.slice(patch.index + 1)];
						break;
					case 'push_back':
						next = [...next, patch_message];
						break;
					case 'push_front':
						next = [patch_message, ...next];
						break;
					case 'remove':
						next = [...next.slice(0, patch.index), ...next.slice(patch.index + 1)];
						break;
					case 'insert':
						next = [...next.slice(0, patch.index), patch_message, ...next.slice(patch.index)];
						break;
					default:
						console.error('UNHANDLED TIMELINE EVENT: ', patch);
						break;
				}
			}
			pendingAnchor = true;
			messages = [...next];
		});
		return () => {
			unlisten.then((v) => v());
		};
	});

	$effect.pre(() => {
		messages.length;
		if (scroller) {
			anchor = getTopIntersectingMessageAnchor(scroller);
		}
	});

	$effect(() => {
		messages.length;
		if (pendingAnchor && anchor && scroller) {
			restoreToMessageAnchor(scroller, anchor);
			pendingAnchor = false;
		}
	});

	const handleLoadTop = async () => {
		if (!scroller || loadTopPending) return;

		loadTopPending = true;
		try {
			await invoke('paginate_up');
		} finally {
			loadTopPending = false;
		}
	};

	const freezeWheel = (e: any) => {
		if (pendingAnchor) {
			e.preventDefault();
			frozenDeltaY += e.deltaY;
		}
	};
	$effect(() => {
		if (!scroller) return;
		scroller.addEventListener('wheel', freezeWheel, { passive: false });
		return () => scroller.removeEventListener('wheel', freezeWheel);
	});

	const onScroll = async () => {
		if (!scroller) return;
		if (scroller.scrollTop <= 200 && !loadTopPending && !scrollLoading) {
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
		<div class="mx-auto pb-[0rem]">
			<Button onclick={handleLoadTop}
				>Load More
				{#if loadTopPending}
					<Spinner />
				{/if}
			</Button>
		</div>
		{#each messages as item, _i (item.key)}
			{#if item.event_id}
				<div class="message" data-key={item.key}>
					<img
						src={users.find((u) => u.id == item.sender)?.avatar_url}
						class="profile"
						alt="fuck"
						loading="lazy"
						decoding="async"
					/>
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
			{/if}
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
		overflow-anchor: none;
	}

	@keyframes fade-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	.message {
		display: flex;
		font-weight: 300;
		gap: 0.5rem;
	}
</style>
