<script>
    import { fetchTrackpoints } from "$lib/api.js";
    import { utcToLocalDate, utcToLocalTime } from "$lib/datetime.js";

    let { curTrack, setTrackpoints } = $props();

    let loader = $derived(load(curTrack));

    async function load(track) {
        if (!track) {
            setTrackpoints(null);
            return null;
        }
        const json = await fetchTrackpoints(track);
        setTrackpoints(json);
        return json;
    }
</script>

{#if curTrack}
    <div class="title">
        <img src="/favicon.svg" alt="" width="16" height="16" />
        <span>
            Track {utcToLocalDate(curTrack.ts_end)}
            · {utcToLocalTime(curTrack.ts_start)}–{utcToLocalTime(
                curTrack.ts_end,
            )}
        </span>
    </div>

    {#await loader}
        <p class="hint">Loading track statistics…</p>
    {:then trackpoints}
        {#if trackpoints && trackpoints.stats}
            {@const s = trackpoints.stats}
            <div class="stats">
                <div class="stat">
                    <span class="k">Device</span>
                    <span class="v"
                        >{curTrack.user_id} / {curTrack.device}</span
                    >
                </div>
                <div class="stat">
                    <span class="k">TID</span>
                    <span class="v">{curTrack.tid}</span>
                </div>
                <div class="stat">
                    <span class="k">Distance</span>
                    <span class="v">{(s.distance / 1000).toFixed(1)} km</span>
                </div>
                <div class="stat">
                    <span class="k">Speed</span>
                    <span class="v"
                        >{s.min_speed}–{s.max_speed} (Ø {Math.round(
                            s.mean_speed,
                        )}) km/h</span
                    >
                </div>
                <div class="stat">
                    <span class="k">Altitude</span>
                    <span class="v"
                        >{s.min_elevation}–{s.max_elevation} (Ø {Math.round(
                            s.mean_elevation,
                        )}) m</span
                    >
                </div>
                <div class="stat">
                    <span class="k">Elevation</span>
                    <span class="v">↗ {s.elevation_up} m ↘ {s.elevation_down} m</span>
                </div>
            </div>
        {/if}
    {:catch error}
        <p class="error">{error.message}</p>
    {/await}
{:else}
    <p class="hint">Select a track to see its statistics.</p>
{/if}

<style>
    .title {
        display: flex;
        align-items: center;
        gap: var(--sp-2);
        font-weight: 600;
        margin-bottom: var(--sp-3);
    }
    .stats {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
        gap: var(--sp-3);
    }
    .stat {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    .stat .k {
        font-size: var(--fs-xs);
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--c-text-faint);
    }
    .stat .v {
        font-size: var(--fs-md);
        font-weight: 500;
    }
</style>
