<script lang="ts">
  import Timeline from "./components/Timeline.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import UserList from "./components/UserList.svelte";
  import type { Message, User } from "./utils/types";
    import Spinner from "$lib/components/ui/spinner/spinner.svelte";
    import { invoke } from "@tauri-apps/api/core";

  $effect(() => {
    (async () => {
      console.log(await invoke("login", {
        homeserver_url: "https://matrix.org"
      }));
    })()
  })

  const users: User[] = [
    {
      name: "SirOlaf",
      img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80"
    },
    {
      name: "Mr Green",
      img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80"
    },
    {
      name: "Click",
      img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80"
    },
    {
      name: "Starr",
      img: "https://img.freepik.com/premium-vector/default-avatar-profile-icon-social-media-user-image-gray-avatar-icon-blank-profile-silhouette-vector-illustration_561158-3407.jpg?semt=ais_user_personalization&w=740&q=80"
    },
  ]

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
    }
  ]
</script>

<main>
  <h4>Users</h4>
  <Button disabled={true} variant="outline">testn butnraeitnoastn </Button>
  <Spinner/>
  <UserList list={users} />
  <Timeline {messages} {users}/>
</main>

<style lang="postcss">
  @reference "tailwindcss";
</style>
