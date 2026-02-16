<script lang="ts">
    import RoomTypeIcon from "./RoomTypeIcon.svelte";

    let {rooms} = $props();
</script>

<div class="roomlist">
    {#each rooms as item}
        <a class="room-elem" href="/room/{item.id}">
            <RoomTypeIcon />

            <p class=name>
                {item.display_name}
            </p>
        </a>
    {/each}
</div>

<style>
    @reference "tailwindcss";

    .roomlist {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding-top: 1rem;
        padding-inline: 0.2rem;
    }

    .room-elem {
        display: grid;
        grid-template-columns: auto 1fr;
        color: theme(--color-gray-500);

        border-radius: 7px;
        gap: 0.5rem;
        font-weight: bold;
        font-size: 1.2rem;
        padding-inline: 1rem;
        padding-block: 0.2rem;
    }

    .room-elem:hover {
        background-color: theme(--color-gray-300);
        cursor: pointer;
    }
</style>
