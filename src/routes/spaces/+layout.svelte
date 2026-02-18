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
    }

    header {
        grid-area: header;
    }

    aside.spaces {
        width: fit-content;
        border-right: 1px solid var(--border);
    }

    aside.rooms {
        grid-area: rooms;
        overflow: auto;
    }

    main {
        grid-area: main;
        overflow: hidden;
    }
</style>
