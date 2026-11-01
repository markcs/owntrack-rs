<script>
    import { fetchTrackpoints } from "$lib/api.js";
    import { utcToLocalDate, utcToLocalTime } from "$lib/datetime.js";

    let { curTrack, setTrackpoints, selectedPoint, clearSelectedPoint } =
        $props();

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

    // Keys we render with a friendly label and a specific order. Anything else
    // (e.g. OwnTracks annotations) is shown afterwards, as-is.
    const KNOWN = [
        ["time", "Time"],
        ["address", "Address"],
        ["speed", "Speed"],
        ["elevation", "Elevation"],
        ["cog", "Course"],
        ["accuracy", "Accuracy"],
        ["v_accuracy", "Vert. accuracy"],
    ];

    function fmtValue(key, value) {
        if (value === null || value === undefined || value === "") return null;
        if (key === "time") return utcToLocalTime(value);
        if (key === "speed") return `${value} km/h`;
        if (key === "elevation") return `${value} m`;
        if (key === "cog") return `${value}°`;
        if (key === "accuracy" || key === "v_accuracy") return `${value} m`;
        return String(value);
    }

    // Build an ordered list of {label, value} rows for the selected point:
    // known fields first (in order), then any remaining properties.
    let pointRows = $derived.by(() => {
        if (!selectedPoint) return [];
        const rows = [];
        const used = new Set();
        for (const [key, label] of KNOWN) {
            const v = fmtValue(key, selectedPoint[key]);
            if (v !== null) rows.push({ label, value: v });
            used.add(key);
        }
        for (const key in selectedPoint) {
            if (used.has(key)) continue;
            const v = selectedPoint[key];
            if (v === null || v === undefined || v === "") continue;
            rows.push({ label: key, value: String(v) });
        }
        return rows;
    });

    let pointAddress = $derived(
        selectedPoint && selectedPoint.address ? selectedPoint.address : null,
    );
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
            <dl class="stats">
                <div class="stat">
                    <dt>Device</dt>
                    <dd>{curTrack.user_id} / {curTrack.device}</dd>
                </div>
                <div class="stat">
                    <dt>TID</dt>
                    <dd>{curTrack.tid}</dd>
                </div>
                <div class="stat">
                    <dt>Distance</dt>
                    <dd>{(s.distance / 1000).toFixed(1)} km</dd>
                </div>
                <div class="stat">
                    <dt>Speed</dt>
                    <dd>
                        {s.min_speed}–{s.max_speed} (Ø{Math.round(
                            s.mean_speed,
                        )}) km/h
                    </dd>
                </div>
                <div class="stat">
                    <dt>Altitude</dt>
                    <dd>
                        {s.min_elevation}–{s.max_elevation} (Ø{Math.round(
                            s.mean_elevation,
                        )}) m
                    </dd>
                </div>
                <div class="stat">
                    <dt>Elevation</dt>
                    <dd>↗{s.elevation_up} ↘{s.elevation_down} m</dd>
                </div>
            </dl>
        {/if}
    {:catch error}
        <p class="error">{error.message}</p>
    {/await}

    {#if selectedPoint}
        <div class="point">
            <div class="point-head">
                <h3>Point details</h3>
                <button
                    class="btn btn-sm btn-ghost"
                    aria-label="Clear selected point"
                    onclick={() => clearSelectedPoint?.()}>✕</button
                >
            </div>
            {#if pointAddress}
                <p class="point-addr">{pointAddress}</p>
            {/if}
            <dl class="point-rows">
                {#each pointRows as row}
                    <div class="prow">
                        <dt>{row.label}</dt>
                        <dd>{row.value}</dd>
                    </div>
                {/each}
            </dl>
        </div>
    {:else}
        <p class="hint point-hint">Click a track point for its full details.</p>
    {/if}
{:else}
    <p class="hint">Select a track to see its statistics.</p>
{/if}

<style>
    .title {
        display: flex;
        align-items: center;
        gap: var(--sp-2);
        font-weight: 600;
        font-size: var(--fs-sm);
        margin-bottom: var(--sp-2);
    }

    /* Compressed stats: two compact columns of key/value pairs. */
    .stats {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
        gap: var(--sp-1) var(--sp-3);
        margin: 0;
    }
    .stat {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: var(--sp-2);
        padding: 2px 0;
        border-bottom: 1px dotted var(--c-border);
    }
    .stat dt {
        font-size: var(--fs-xs);
        text-transform: uppercase;
        letter-spacing: 0.03em;
        color: var(--c-text-faint);
        margin: 0;
    }
    .stat dd {
        margin: 0;
        font-size: var(--fs-sm);
        font-weight: 500;
        text-align: right;
    }

    .point-hint {
        margin-top: var(--sp-3);
    }

    .point {
        margin-top: var(--sp-3);
        padding-top: var(--sp-2);
        border-top: 1px solid var(--c-border);
    }
    .point-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }
    .point-head h3 {
        font-size: var(--fs-sm);
        text-transform: uppercase;
        letter-spacing: 0.03em;
        color: var(--c-text-muted);
    }
    .point-addr {
        margin: var(--sp-1) 0 var(--sp-2);
        font-weight: 600;
        font-size: var(--fs-sm);
    }
    .point-rows {
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 1px;
    }
    .prow {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: var(--sp-2);
        padding: 2px 0;
        border-bottom: 1px dotted var(--c-border);
    }
    .prow dt {
        margin: 0;
        font-size: var(--fs-xs);
        color: var(--c-text-faint);
        white-space: nowrap;
    }
    .prow dd {
        margin: 0;
        font-size: var(--fs-sm);
        text-align: right;
        word-break: break-word;
    }
</style>
