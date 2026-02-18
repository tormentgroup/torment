<script lang="ts">
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import RoomTypeIcon from '$lib/components/RoomTypeIcon.svelte';
	import { page } from '$app/state';
	import type { RoomInfoMinimal } from '$lib/utils/types';

	let { rooms, pending, roomId }: { rooms: RoomInfoMinimal[]; pending?: boolean; roomId?: string } =
		$props();
</script>

{#if pending}
	<div class="roomlist">
		<Spinner />
	</div>
{:else}
	<div class="roomlist">
		{#each rooms as room}
			<div>
				<a
					class={`room-elem ${roomId == room.room_id && 'selected'}`}
					href="/spaces/{page.params.spaceId}/room/{room.room_id}"
				>
					{#if room.avatar_url}
                        <img src={room.avatar_url} width={20} height={20} class="avatar" />
                    {:else}
						<RoomTypeIcon />
					{/if}

					<p class="name">
						{room.display_name}
					</p>
				</a>
				{#each room.children as inner_room}
					<a
						class={`room-elem inner ${roomId == inner_room.room_id && 'selected'}`}
						href="/spaces/{page.params.spaceId}/room/{inner_room.room_id}"
					>
						<RoomTypeIcon />

						<p class="name">
							{inner_room.display_name}
						</p>
					</a>
				{/each}
			</div>
		{/each}
	</div>
{/if}

<style>
	@reference "tailwindcss";

    .avatar {
        aspect-ratio: 1/1;
        object-fit: cover;
        object-position: center;
        border-radius: 5px;
    }

	.roomlist {
		display: flex;
		border-right: 1px solid var(--border);
		height: 100%;
		flex-direction: column;
		padding-top: 1rem;
		padding-inline: 0.2rem;
		gap: 0.1rem;
		overflow: hidden;
		background: var(--sidebar);
	}

	.inner {
		margin-left: 1rem;
	}

	.room-elem {
		display: grid;
		grid-template-columns: auto 1fr;
		color: var(--color-gray-500);

		border-radius: 7px;
		gap: 0.5rem;
		font-weight: bold;
		font-size: 0.9rem;
		padding: 0.2rem 0.5rem;
	}

	.room-elem p {
		text-overflow: ellipsis;
		white-space: nowrap;
		width: 100%;
		overflow: hidden;
	}

	.room-elem:hover {
		background-color: var(--popover);
		cursor: pointer;
	}

	.selected {
		background: var(--muted);
	}

	.selected:hover {
		background: var(--muted);
	}
</style>
