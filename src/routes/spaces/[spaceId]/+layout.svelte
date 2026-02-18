<script lang="ts">
	import { page } from '$app/state';
	import type { RoomInfoMinimal } from '$lib/utils/types';
	import RoomList from './RoomList.svelte';
	import RoomHeader from './RoomHeader.svelte';
	import { invoke } from '@tauri-apps/api/core';

	let { children } = $props();
	let spaceId = $derived(page.params.spaceId);
	let roomId = $derived(page.params.roomId);
	let pending = $state(true);
    let requestId = 0;

	let rooms: RoomInfoMinimal[] = $state([]);
	const updateRooms = async () => {
        const id = ++requestId;
        pending = true;
        rooms = [];
		try {
			let l = (await invoke('get_rooms', { space_id: spaceId })) as RoomInfoMinimal[];
			console.log(l);
			for (let room of l) {
				console.log(room.children_count);
				if (room.children_count > 0) {
                    room.children = await invoke("get_rooms", {space_id: room.room_id}) as RoomInfoMinimal[];
				}
			}
            if (id != requestId) {
                return;
            }
			rooms = l;
		} finally {
			pending = false;
		}
	};
	$effect(() => {
		if (spaceId) {
			updateRooms();
		}
	});
	let activeRoom = $derived(rooms.find((i) => i.room_id == page.params.roomId));
</script>

<div class="layout">
	<header>
		<RoomHeader {activeRoom} />
	</header>

	<aside class="rooms">
		<RoomList {rooms} {pending} {roomId} />
	</aside>

	<main>
		{@render children()}
	</main>
</div>

<style>
	@reference "tailwindcss";

	.layout {
		display: grid;
		grid-template-columns: auto auto 1fr;
		grid-template-rows: auto 1fr;
		grid-template-areas:
			'spaces header header'
			'spaces rooms main';

		height: 100vh;

		background-color: var(--color-gray-300);
		gap: 1px;
	}

	header {
		grid-area: header;
	}

	aside.rooms {
		grid-area: rooms;
		background-color: white;
		overflow: auto;
		width: 20rem;
	}

	main {
		grid-area: main;
		overflow: hidden;
	}
</style>
