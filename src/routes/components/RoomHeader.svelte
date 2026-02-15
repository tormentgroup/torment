<script lang="ts">
    import type { RoomInfo } from "../utils/types";
    import RoomTypeIcon from "./RoomTypeIcon.svelte";

    let {activeRoom}: {activeRoom: RoomInfo} = $props();
</script>

<div class="layout">
    <RoomTypeIcon />

    <div class="breadcrumb">
        <p>{activeRoom.parent}</p>
        <p>&gt</p>
        <p>{activeRoom.name}</p>
    </div>
</div>

<style>
    @reference "tailwindcss";

    .layout {
        display: grid;
        grid-template-columns: auto 1fr;
        gap: 0.5rem;
        padding: 0.3rem 0rem 0.3rem 1rem;
        font-size: 1.3rem;
        background-color: white;
        color: theme(--color-gray-400);
    }

    .breadcrumb {
        display: flex;
        flex-direction: row;
        gap: 0.5rem;
    }

    .breadcrumb > p:last-child {
        color: theme(--color-gray-500);
    }
</style>