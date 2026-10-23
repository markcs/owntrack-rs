<script>
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import { trackQuery } from "./datetime.js";

    let { curTrack, setTrackpoints } = $props();
    let loader = $derived(load_trackpoints(curTrack));

    async function load_trackpoints(track) {
        if (!track) return null;
        const res = await fetch(
            `${PUBLIC_BASE_URL}/trackpoints?${trackQuery(track)}`,
        );
        const json = await res.json();
        setTrackpoints(json);
        return json;
    }
</script>

{#await loader then trackpoints}
    {#if trackpoints && curTrack}
        {@const track = trackpoints.stats}
        <dl class="stats">
            <dt>tid</dt>
            <dd><b>{curTrack.tid}</b></dd>
            <dt>User</dt>
            <dd>{curTrack.user_id}</dd>
            <dt>Device</dt>
            <dd>{curTrack.device}</dd>
            <dt>Distance</dt>
            <dd>{(track.distance / 1000).toFixed(1)} km</dd>
            <dt>Speed</dt>
            <dd>
                {track.min_speed}–{track.max_speed} (Ø {Math.round(
                    track.mean_speed,
                )}) km/h
            </dd>
            <dt>Altitude</dt>
            <dd>
                {track.min_elevation}–{track.max_elevation} (Ø {Math.round(
                    track.mean_elevation,
                )}) m
            </dd>
            <dt>Elevation</dt>
            <dd>↗{track.elevation_up} m ↘{track.elevation_down} m</dd>
        </dl>
    {/if}
{:catch error}
    <p class="error">{error.message}</p>
{/await}

<style>
    .stats {
        display: grid;
        grid-template-columns: auto 1fr;
        gap: 4px 16px;
        margin: 0;
        padding: 8px 0;
        font-size: 0.875rem;
    }

    .stats dt {
        color: var(--color-text-secondary);
        white-space: nowrap;
    }

    .stats dd {
        margin: 0;
        color: var(--color-text);
        text-align: right;
    }

    .error {
        margin: 0;
        padding: 8px 0;
        color: #d32f2f;
        font-size: 0.875rem;
    }
</style>
