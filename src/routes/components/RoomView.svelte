<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type { Message, User } from '../../lib/utils/types';
	import Timeline from './Timeline.svelte';
	import UserList from './UserList.svelte';

	let { roomId }: { roomId: string } = $props();

	let users: User[] = $state([]);
	let pending = $state(true);
	let timelinePending = $state(true);
	let messages: Message[] = $state([]);

	$effect(() => {
		(async () => {
			messages = [];
			users = [];
			pending = true;
			timelinePending = true;
			let membersAsync = invoke('get_members', { roomId });
			membersAsync
				.then((m) => {
					users = m as any;
				})
				.finally(() => {
					pending = false;
				});

			let mAsync = invoke('open_room', { room_id: roomId });
			mAsync
				.then((m) => {
					messages = m as Message[];
					console.log(m);
				})
				.finally(() => {
					timelinePending = false;
				});
		})();
	});

    const handleSubmit = async (formEvent: SubmitEvent) => {
        formEvent.preventDefault();
        if (!formEvent.currentTarget) {
            return
        }
        const form = formEvent?.currentTarget as HTMLFormElement;
        const data = new FormData(form);
        const message = data.get("message");
        if (!message) {
            return;
        }
        await invoke("send_message", {room_id: roomId, message: message})
        form.reset();
    }
</script>

<div class="layout">
	<div class="chat">
		<Timeline {users} {messages} pending={timelinePending} />
        <form onsubmit={handleSubmit} class="message-composer-wrapper">
            <input name="message" type="text" class="message-composer" placeholder="Send an unencrypted message..." />
        </form>
	</div>

	<aside>
		<UserList {users} {pending} />
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

	.message-composer-wrapper {
        width: 100%;
    }
	.message-composer {
		padding: 0.9rem;
        width: 100%;
		border-radius: 5px;
		background: var(--input);
		font-weight: 500;
		font-size: 0.9rem;
	}
</style>
