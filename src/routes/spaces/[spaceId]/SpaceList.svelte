<script lang="ts">
    import type { RoomInfoMinimal } from "$lib/utils/types";

    let {spaces}: {spaces: RoomInfoMinimal[]} = $props();
</script>

<div class="layout">
    {#each spaces as item}
        <div class="space-elem">
            <div class="hover-bar"></div>
            <a href="/spaces/{item.room_id}">
                <img src={item.avatar_url} width={50} height={50} alt={item.display_name}/>
            </a>
        </div>
    {/each}
</div>

<style>
    @reference "tailwindcss";

    .layout {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding-top: 1rem;
    }

    .space-elem {
        position: relative;
        display: grid;
        grid-template-columns: auto auto 1fr;
        align-items: center;

        gap: 0.5rem;
        font-weight: bold;
        font-size: 1.2rem;
        padding-left: 1rem;
    }

    .space-elem:hover {
        cursor: pointer;
    }

    .hover-bar {
        position: absolute;
        width: 5px;
        height: 60%;
        border-radius: 0 4px 4px 0;
        background-color: theme(--color-gray-500);
        transform: scale(0);
        transition: transform 0.15s ease-in-out;
    }

    .space-elem:hover > .hover-bar {
        transform: scale(1);
    }
</style>
