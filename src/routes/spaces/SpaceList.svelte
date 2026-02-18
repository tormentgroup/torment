<script lang="ts">
    import type { SpaceInfoMinimal } from "$lib/utils/types";

    let {spaces}: {spaces: SpaceInfoMinimal[]} = $props();
</script>

<div class="layout">
    {#each spaces as item}
        <div class="space-elem">
            <div class="hover-bar"></div>
            <a href={`/spaces/${item.room_id}`}>
                <img src={item.avatar_url} width={40} height={40} alt={item.display_name}/>
            </a>
        </div>
    {/each}
</div>

<style>
    @reference "tailwindcss";

    .layout {
        display: flex;
        flex-direction: column;
        background: var(--sidebar);
        height: 100%;
        gap: 0.5rem;
        padding: 1rem 0;
    }

    .space-elem {
        position: relative;
        display: flex;
        align-items: center;

        padding-inline: 0.5rem;
        gap: 0.5rem;
        font-weight: bold;
        font-size: 1.2rem;
    }

    img {
        object-fit: cover;
        object-position: center;
        border-radius: 10px;
        aspect-ratio: 1/1;
    }

    .space-elem:hover {
        cursor: pointer;
    }

    .hover-bar {
        position: absolute;
        left: 0;
        width: 5px;
        height: 60%;
        border-radius: 0 4px 4px 0;
        background-color: var(--border);
        transform: scale(0);
        transition: transform 0.15s ease-in-out;
    }

    .space-elem:hover > .hover-bar {
        transform: scale(1);
    }
</style>
