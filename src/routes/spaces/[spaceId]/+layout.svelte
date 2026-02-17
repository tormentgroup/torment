<script lang="ts">
    import { page } from "$app/state";
    import type { RoomInfoMinimal } from "$lib/utils/types";
    import RoomList from "./RoomList.svelte";
    import RoomHeader from "./RoomHeader.svelte";
    import SpaceList from "./SpaceList.svelte";
    import { invoke } from "@tauri-apps/api/core";

    let { children } = $props();

    let allRooms: RoomInfoMinimal[] = $state([]);
    invoke("get_rooms").then((l: RoomInfoMinimal[]) => {
        console.log(l);
        allRooms = l;
    });

    let topLevelSpaces: RoomInfoMinimal[] = $derived(
        allRooms.filter(r => r.is_space && r.parent_ids.length === 0)
    )

    let rooms: RoomInfoMinimal[] = $derived(
        allRooms.filter(r => r.parent_ids.includes(page.params.spaceId ? page.params.spaceId : ""))
    );

    let activeRoom = $derived(rooms.find((r) => r.room_id === page.params.roomId));
</script>

<div class="layout">
    <aside class="spaces">
        <SpaceList spaces={topLevelSpaces} />
    </aside>

    <header>
        {#if activeRoom}
            <RoomHeader {activeRoom} />
        {/if}
    </header>

    <aside class="rooms">
        <RoomList {rooms} />
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
            "spaces header header"
            "spaces rooms main";

        height: 100vh;

        background-color: theme(--color-gray-300);
        gap: 1px;
    }

    header {
        grid-area: header;
    }

    aside.spaces {
        grid-area: spaces;
        background-color: white;
    }

    aside.rooms {
        grid-area: rooms;
        background-color: white;
        overflow: auto;
    }

    main {
        grid-area: main;
        overflow: hidden;
    }
</style>
