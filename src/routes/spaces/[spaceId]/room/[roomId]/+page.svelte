<script lang="ts">
    import { page } from '$app/state';
    import RoomView from "../../../../components/RoomView.svelte";

    let roomId = $derived(page.params.roomId ? page.params.roomId : "");
</script>

<RoomView {roomId}/>
