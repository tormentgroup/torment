<script lang="ts">
    import type { Message, User } from "../../lib/utils/types";
    import { formatDate } from "../../lib/utils/utils";
    let {messages, users}: {messages: Message[], users: User[]}= $props();
</script>

<div class="wrapper">
    <!--
    {#each messages as item}
        <div class="message">
            <img src={users[item.userIndex].avatar_url} class="profile" alt="fuck"/>
            <div class="inner-wrapper">
                <div class="title">
                    <p class="name">{users[item.userIndex].display_name}</p>
                    {formatDate(new Date(item.timestamp))}
                </div>
                <p>{item.message}</p>
            </div>
        </div>
    {/each}
    -->
</div>

<style>

p {
    margin-block: 0.5rem;
}

.title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.name {
    font-weight: 900;
}

.profile {
    object-fit: cover;
    object-position: center;
    max-width: 3rem;
    aspect-ratio: 1/1;
}

.wrapper {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
    justify-content: flex-end;
    height: 100%;
}

.message {
    display: flex;
    gap: 1rem;
}
</style>
