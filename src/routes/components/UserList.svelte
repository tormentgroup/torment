<script>
    let {list} = $props();
</script>

<div>
  <div class="wrapper">
    {#each list as item}
      <div class="profile">
        <img src={item.img} width={50} height={50} class="profile-img"/>
        <p class="user-name">{item.name}</p>
      </div>
    {/each}
  </div>
</div>

<style>
.user-name {
  font-size: 1rem;
  font-weight: 400;
}
.wrapper {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  background-color: white;
}
.profile {
  display: flex;
  align-items: center;
  gap: 1rem;
}
.profile:nth-of-type(1) {
  border-top: none;
}
.profile-img {
  object-fit: cover;
  object-position: center;
}
</style>
