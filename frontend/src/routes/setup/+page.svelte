<script>
    import { onMount } from "svelte";
    import { fetchOtrc, otrcDownloadUrl } from "$lib/api.js";

    let status = $state("loading");
    let otrc = $state(null);

    onMount(async () => {
        const result = await fetchOtrc();
        status = result.status;
        otrc = result.otrc ?? null;
    });
</script>

<div class="setup">
    <section class="card setup-card">
        <h2>Device setup</h2>

        {#if status === "loading"}
            <p class="hint">Loading setup configuration…</p>
        {:else if status === "ok"}
            <p class="muted">
                Configure the OwnTracks app on your device, or download the
                remote-configuration file.
            </p>
            <div class="actions">
                <a
                    class="btn btn-primary"
                    href="owntracks:///config?inline={otrc}"
                >
                    Set up OwnTracks app
                </a>
                <a class="btn" href={otrcDownloadUrl()} download="otrc.json">
                    Download OTRC file
                </a>
            </div>
            <p class="hint note">
                The button opens the OwnTracks app with this server's settings.
                If nothing happens, install the app first, then download the
                OTRC file and import it manually.
            </p>
        {:else if status === "invalid"}
            <p class="error">
                Authorization failed or the configuration has expired.
            </p>
        {:else}
            <p class="error">
                An error occurred while loading the setup configuration.
            </p>
        {/if}
    </section>
</div>

<style>
    .setup {
        padding: var(--sp-5) var(--sp-4);
        display: flex;
        justify-content: center;
    }
    .setup-card {
        width: 100%;
        max-width: 540px;
    }
    .actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--sp-3);
        margin-top: var(--sp-4);
    }
    .note {
        margin-top: var(--sp-4);
    }
</style>
