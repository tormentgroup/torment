<script lang="ts">
    import RoomTypeIcon from "./RoomTypeIcon.svelte";

    let {rooms} = $props();
</script>

<div class="roomlist">
    {#each rooms as item}
        <div class="room-elem">
            <RoomTypeIcon />

            <p class=name>
                {item.name}
            </p>
        </div>
    {/each}
</div>

<style>
    @reference "tailwindcss";

    .roomlist {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding-top: 1rem;
    }

    .room-elem {
        display: grid;
        grid-template-columns: auto 1fr;
        color: theme(--color-gray-500);

        gap: 0.5rem;
        font-weight: bold;
        font-size: 1.2rem;
        padding-left: 1rem;
    }
</style>
