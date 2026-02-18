<script>
	let { list } = $props();
</script>

<div class="wrapper">
	<p class="users-count">Users &mdash; {list.length}</p>
	{#each list as item}
		<div class="profile-wrapper">
			<div class="profile">
				<img src={item.img} width={35} height={35} class="profile-img" />
				<p class="user-name">{item.name}</p>
			</div>
		</div>
	{/each}
</div>

<style>
	@reference "tailwindcss";
	.user-name {
		font-size: 0.8rem;
		font-weight: 500;
	}
	.users-count {
		font-size: 0.8rem;
	}
	.wrapper {
		display: flex;
		flex-direction: column;
		margin: 0.8rem 0.5rem;
		overflow: auto;
	}

	.profile-wrapper {
		padding: 0.1rem 0;
		width: 100%;
	}

	.profile {
		display: flex;
		align-items: center;
		padding: 0.3rem;
		width: 100%;
		border-radius: 8px;
		gap: 0.5rem;
	}

	.profile-wrapper:hover .profile {
		cursor: pointer;
	}

	.profile-img {
		object-fit: cover;
		object-position: center;
		border-radius: 100%;
	}
</style>
