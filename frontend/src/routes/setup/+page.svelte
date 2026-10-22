<script>
    import "../global.css";
    import "../owntracks-theme.css";
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import { onMount } from "svelte";

    let loading = $state("loading");
    let otrc = $state();

    onMount(async () => {
        try {
            const res = await fetch(`${PUBLIC_BASE_URL}/otrc`);
            if (res.status == 403) {
                loading = "invalid";
            } else if (res.ok) {
                const json = await res.json();
                // Base64 encode the JSON
                const jsonString = JSON.stringify(json);
                otrc = btoa(jsonString);
                loading = "ok";
            } else {
                loading = "error";
            }
        } catch (err) {
            console.log(err);
            loading = "error";
        }
    });
</script>

<div class="page">
    <header class="ot-header">
        <span class="header-title">Setup</span>
        <div class="ot-nav-item header-right">
            <a class="button button-flat" href="{PUBLIC_BASE_URL}/">
                &larr; Back to map
            </a>
        </div>
    </header>

    <main>
        <section class="ot-card setup-card">
            {#if loading === "loading"}
                <p class="hint">Loading setup configuration…</p>
            {:else if loading === "ok"}
                <p>Configure the OwnTracks app on your device:</p>
                <div class="setup-actions">
                    <a
                        class="button button-primary"
                        href="owntracks:///config?inline={otrc}"
                    >
                        Setup OwnTracks App
                    </a>
                    <a
                        class="button"
                        href="{PUBLIC_BASE_URL}/otrc"
                        download="otrc.json"
                    >
                        Download OTRC file
                    </a>
                </div>
            {:else if loading === "invalid"}
                <p class="error">
                    Authorization failed or configuration has expired.
                </p>
            {:else if loading === "error"}
                <p class="error">
                    An error occurred while loading the setup configuration.
                </p>
            {/if}
        </section>
    </main>
</div>

<style>
    .page {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
    }
    main {
        flex: 1;
        padding: 20px;
    }
    .setup-card {
        max-width: 520px;
    }
    .setup-actions {
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
        margin-top: 12px;
    }
    .button {
        border: 1px solid var(--color-separator);
        text-decoration: none;
    }
    .button.button-primary {
        border-color: var(--color-primary);
    }
    .hint {
        color: #666;
    }
    .error {
        color: #c00;
    }
</style>
