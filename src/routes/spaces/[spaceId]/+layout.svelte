<script lang="ts">
    import { page } from "$app/state";
    import { invoke } from "@tauri-apps/api/core";
    import Spinner from "$lib/components/ui/spinner/spinner.svelte";
    import { goto } from "$app/navigation";
    import type { RoomInfo, SpaceInfo } from "$lib/utils/types";
    import RoomList from "./RoomList.svelte";
    import RoomHeader from "./RoomHeader.svelte";
    import SpaceList from "./SpaceList.svelte";

    let { children } = $props();
    let pending = $state(true);

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

    async function ensureAuth() {
        try {
            await invoke("login", { homeserver_url: "https://matrix.org" });
            // FIXME: Need to check state. If we are now logged in, we can continue, otherwise we send user to login screen
            pending = false;
        } catch (e: any) {
            switch (e.type) {
                case "InvalidState":
                    if (page.url.pathname == "/auth/login") {
                        await goto("/");
                    }
                    pending = false;
                    break;
                default:
                    // FIXME: handle case where we are currently in progress but not in login page because login may fail
                    // FIXME: handle case where we failed auto login and must send user to login page
                    pending = false;
                    console.log(e);
                    await goto("/auth/login");
                    break;
            }
        }
    }

    $effect(() => {
        // NOTE: Need this to re-run effect every page load
        page.url.href;

        console.log(page.url.href);

        void ensureAuth();
    });
</script>

{#if !pending}
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
{:else}
    <Spinner />
{/if}

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
