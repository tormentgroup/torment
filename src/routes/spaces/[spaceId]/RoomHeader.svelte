<script lang="ts">
	import type { RoomInfoMinimal } from '../../../lib/utils/types';
	import RoomTypeIcon from '$lib/components/RoomTypeIcon.svelte';

	let { activeRoom }: { activeRoom?: RoomInfoMinimal } = $props();
	$effect(() => {
		console.log(activeRoom, "ANOTNRUYONT YURNTYUO")
	})
</script>

<div class="layout">
	<RoomTypeIcon />

	<div class="breadcrumb">
		<p>Torment Nexus</p>
		{#if activeRoom}
			<p>&gt</p>
			<p>{activeRoom.display_name}</p>
		{/if}
	</div>
</div>

<style>
	@reference "tailwindcss";

	.layout {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.5rem;
		padding: 0.3rem 0rem 0.3rem 1rem;
		font-size: 1.1rem;
		background-color: white;
		color: var(--color-gray-400);
	}

	.breadcrumb {
		display: flex;
		flex-direction: row;
		gap: 0.5rem;
	}

	.breadcrumb > p:last-child {
		color: var(--color-gray-500);
	}
</style>

