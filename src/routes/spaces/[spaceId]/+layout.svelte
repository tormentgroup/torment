<script lang="ts">
    import { page } from "$app/state";
    import type { RoomInfo, SpaceInfo } from "$lib/utils/types";
    import RoomList from "./RoomList.svelte";
    import RoomHeader from "./RoomHeader.svelte";
    import SpaceList from "./SpaceList.svelte";

    let { children } = $props();

    let spaces: SpaceInfo[] = [
        {
            name: "Purgatory",
            id: "1",
            img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
        },
        {
            name: "Torment Nexus",
            id: "2",
            img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
        },
    ];

    let rooms: RoomInfo[] = $derived.by(() => {
        if (page.params.spaceId == "1") {
            return [
                {
                    display_name: "general",
                    id: "1",
                    kind: "",
                },
                {
                    display_name: "general2",
                    id: "2",
                    kind: "",
                },
            ];
        } else if (page.params.spaceId == "2") {
            return [
                {
                    display_name: "gamers",
                    id: "3",
                    kind: "",
                },
                {
                    display_name: "nerds",
                    id: "4",
                    kind: "",
                },
            ]
        }
        return [];
    });

    let activeRoom = $derived(rooms.find((r) => r.id === page.params.roomId));

</script>

<div class="layout">
    <aside class="spaces">
        <SpaceList {spaces} />
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
    }

    main {
        grid-area: main;
        overflow: hidden;
    }
</style>
