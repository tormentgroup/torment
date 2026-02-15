<script lang="ts">
    import type { Message, RoomInfo, User } from "../utils/types";
    import RoomHeader from "./RoomHeader.svelte";
    import RoomList from "./RoomList.svelte";
    import Timeline from "./Timeline.svelte";
    import UserList from "./UserList.svelte";

    const users: User[] = [
        {
            name: "SirOlaf",
            img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
        },
        {
            name: "Mr Green",
            img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
        },
        {
            name: "Click",
            img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
        },
        {
            name: "Starr",
            img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80",
        },
    ];

    const messages: Message[] = [
        {
            userIndex: 0,
            message: "Yo",
            timestamp: Date.now() - 160000,
        },
        {
            userIndex: 1,
            message: "Hi",
            timestamp: Date.now() - 1000,
        },
        {
            userIndex: 2,
            message: "Hi",
            timestamp: Date.now() - 0,
        },
        {
            userIndex: 3,
            message: "ITRSNtinrsoiatniaotraitarntnaritnarint irasntiars",
            timestamp: Date.now() - 0,
        },
    ];

    const rooms: RoomInfo[] = [
        {
            name: "general",
            parent: "Purgatory"
        },
        {
            name: "general2",
            parent: "Purgatory"
        }
    ];
</script>

<div class="layout">
    <header>
        <RoomHeader activeRoom={rooms[1]}/>
    </header>

    <aside>
        <RoomList {rooms}/>
    </aside>

    <div class="chat">
        <Timeline {users} {messages}/>
        <input type="text" class="message-composer" placeholder="Send an unencrypted message..." />
    </div>

    <aside>
        <UserList list={users}/>
    </aside>
</div>

<style>
    @reference "tailwindcss";

    header {
        grid-column: 1 / -1;
    }

    aside {
        min-width: 12rem;
        overflow-y: auto;
        grid-row: 2 / -1;
        background-color: white;
    }

    aside:first-of-type {
        grid-column: 1;
    }

    aside:last-of-type {
        grid-column: 3;
    }

    .layout {
        display: grid;
        grid-template-columns: auto 1fr auto;
        grid-template-rows: auto 1fr;
        column-gap: 5rem;
        row-gap: 1rem;
        height: 100vh;
        padding: 0.2rem;
        overflow: hidden;

        background-color: theme(--color-gray-300);
        gap: 1px;
    }

    .chat {
        display: flex;
        flex-direction: column;
        overflow: hidden;
        background-color: white;
        padding: 0.5rem;
    }

    .message-composer {
        background-color: theme(--color-gray-200);
        padding: 0.8rem;
        border-radius: 5px;
    }
</style>
