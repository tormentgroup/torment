<script lang="ts">
    import type { RoomInfo, SpaceInfo } from "./utils/types";
    import RoomHeader from "./components/RoomHeader.svelte";
    import RoomList from "./components/RoomList.svelte";
    import SpaceList from "./components/SpaceList.svelte";
    import "../app.css";

    let { children } = $props();

    let spaces: SpaceInfo[] = [{
        name: "Purgatory",
        img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
    }, {
        name: "Torment Nexus",
        img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
    }];

    let rooms: RoomInfo[] = [{
        display_name: "general",
        id: "1",
        kind: "",
    }, {
        display_name: "general2",
        id: "2",
        kind: "",
    }];

    let activeRoom = rooms[0];
</script>

<div class="layout">
    <aside class="spaces">
        <SpaceList {spaces}/>
    </aside>

    <header>
        <RoomHeader {activeRoom}/>
    </header>

    <aside class="rooms">
        <RoomList {rooms}/>
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
