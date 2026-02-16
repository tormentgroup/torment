<script lang="ts">
    let {spaces} = $props();
</script>

<div class="layout">
    {#each spaces as item}
        <div class="space-elem">
            <img src={item.img} width={50} height={50} alt={item.name}/>
            <p class=name>
                {item.display_name}
            </p>
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
        display: grid;
        grid-template-columns: auto 1fr;
        color: theme(--color-gray-500);

        gap: 0.5rem;
        font-weight: bold;
        font-size: 1.2rem;
        padding-left: 1rem;
    }
</style>
