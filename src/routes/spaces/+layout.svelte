<script lang="ts">
    import type { SpaceInfoMinimal } from "$lib/utils/types";
    import SpaceList from "./SpaceList.svelte";
    import { invoke } from "@tauri-apps/api/core";

    let { children } = $props();

    let spaces: SpaceInfoMinimal[] = $state([]);
    $effect(() => {
        invoke("get_spaces").then((l: SpaceInfoMinimal[]) => {
            console.log(l);
            spaces = l;
        });
    })

</script>

<div class="layout">
    <aside class="spaces">
        <SpaceList {spaces} />
    </aside>

        {@render children()}
</div>

<style>
    .layout {
        display: grid;
        grid-template-columns: auto 1fr;
        height: 100vh;
        background-color: var(--color-gray-300);
    }

    header {
        grid-area: header;
    }

    aside.spaces {
        background-color: white;
        width: fit-content;
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
