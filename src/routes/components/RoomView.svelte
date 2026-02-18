<script lang="ts">
    import type { Message, User } from "../../lib/utils/types";
    import Timeline from "./Timeline.svelte";
    import UserList from "./UserList.svelte";

    let {roomId}: {roomId: string} = $props();

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

    let messages: Message[] = $derived.by(() => {
        if (roomId == "1") {
            return [
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
        } else if (roomId == "2") {
            return [{
                userIndex: 0,
                message: "Gaming",
                timestamp: Date.now() - 1000,
            }];
        } else if (roomId == "3") {
            return [{
                userIndex: 1,
                message: "Gaming happens here",
                timestamp: Date.now() - 1000,
            }]
        } else if (roomId == "4") {
            return [{
                userIndex: 3,
                message: "Official moontie user meetup",
                timestamp: Date.now() - 1000,
            }]
        }
        return [];
    });
</script>

<div class="layout">
    <div class="chat">
        <Timeline {users} {messages}/>
        <input type="text" class="message-composer" placeholder="Send an unencrypted message..." />
    </div>

    <aside>
        <UserList list={users}/>
    </aside>
</div>

<style>
    aside {
        width: 15rem;
        overflow: auto;
        grid-row: 2 / -1;
        background-color: var(--background);
        color: var(--color-gray-400);
        border-left: 1px solid var(--border);
    }

    aside:last-of-type {
        grid-column: 2;
    }

    .layout {
        display: grid;
        grid-template-columns: 1fr auto;
        height: 100%;
        overflow: hidden;

        background-color: var(--background);
        gap: 1px;
    }

    .chat {
        display: flex;
        flex-direction: column;
        overflow: hidden;
        padding: 0.5rem;
    }

    .message-composer {
        padding: 0.9rem;
        border-radius: 5px;
        background: var(--input);
        font-weight: 500;
        font-size: .9rem;
    }
</style>
