<script lang="ts">
    import { page } from "$app/state";
    import { invoke } from "@tauri-apps/api/core";
    import "../app.css";
    import { goto } from "$app/navigation";
    import { listen } from "@tauri-apps/api/event";
    import { Spinner } from "$lib/components/ui/spinner";
    let { children } = $props();

    let pending = $state(true);

    async function ensureAuth() {
        try {
            let url = await invoke('login', { homeserver_url: 'https://matrix.org' });
            // NOTE: if login was success, that only means that the url was launched
            if (page.url.pathname != "/auth/login") {
                await goto(`/auth/login?${url}`);
            }
            // FIXME: Need to check state. If we are now logged in, we can continue, otherwise we send user to login screen
        } catch (e: any) {
            if (e.InvalidState == "Complete") {
                if (page.url.pathname == "/auth/login") {
                    await goto("/");
                }
            } else {
                // FIXME: handle case where we are currently in progress but not in login page because login may fail
                // FIXME: handle case where we failed auto login and must send user to login page
                pending = false;
                if (page.url.pathname == "/auth/login") {
                    return;
                }
                console.log(e);
                await goto("/auth/login");
            }
        }
    }


    // Listeners only need to be set up once
    $effect(() => {
        const unlisteners = [
            listen("login-error", (e: any) => {
                console.log(e);
                if (page.url.pathname != "/auth/login") {
                    goto("/auth/login")
                }
            }),
            listen("sync-ready", () => {
                console.log("Sync ready")
                pending = false;
            }),
        ];

        return () => {
            unlisteners.map((x) => x.then((f) => f()));
        };
    });

    // Auth check re-runs on every navigation
    $effect(() => {
        page.url.href;
        void ensureAuth();
    });
</script>

{#if !pending}
{@render children()}
{:else}
    <Spinner />
{/if}

<style lang="postcss">
    @reference "tailwindcss";
</style>
