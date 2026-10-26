<script>
    import {
        addDays,
        isoDateString,
        isToday,
        utcToLocalTime,
    } from "$lib/datetime.js";
    import {
        fetchTrackInfos,
        gpxDownloadUrl,
        geoJsonDownloadUrl,
        csvDownloadUrl,
    } from "$lib/api.js";

    let { date, curTrack, setDate, setCurTrack, preselectDeviceId } = $props();

    let datestr = $derived(isoDateString(date));
    let loader = $derived(loadInfos(datestr));

    // Device id to auto-select once, when arriving via a deep link.
    let pendingPreselect = $state(preselectDeviceId ?? null);
    let openMenu = $state(null);

    async function loadInfos(_datestr) {
        const json = await fetchTrackInfos(date, datestr);
        if (pendingPreselect != null) {
            const match = json.find((t) => t.device_id === pendingPreselect);
            pendingPreselect = null;
            if (match) {
                setCurTrack(match);
                return json;
            }
        }
        // Auto-select the first track if it's the only one for the first day.
        if (
            json.length === 1 ||
            (json.length > 1 &&
                isoDateString(json[0].ts_end) !== isoDateString(json[1].ts_end))
        ) {
            setCurTrack(json[0]);
        } else {
            setCurTrack(null);
        }
        return json;
    }

    function isSelected(track) {
        return (
            curTrack &&
            curTrack.device_id === track.device_id &&
            isoDateString(curTrack.ts_start) === isoDateString(track.ts_start)
        );
    }

    function toggleMenu(key, event) {
        event.stopPropagation();
        openMenu = openMenu === key ? null : key;
    }
    function trackKey(t) {
        return `${t.device_id}-${t.ts_start}`;
    }
</script>

<svelte:window onclick={() => (openMenu = null)} />

<div class="tracklist">
    <div class="datebar">
        <button
            class="btn btn-sm"
            aria-label="Previous day"
            onclick={() => setDate(addDays(date, -1))}>‹</button
        >
        <input
            class="input date"
            type="date"
            value={datestr}
            onchange={(e) => {
                if (e.target.value) {
                    const [y, m, d] = e.target.value.split("-").map(Number);
                    setDate(new Date(y, m - 1, d));
                }
            }}
        />
        <button
            class="btn btn-sm"
            aria-label="Next day"
            onclick={() => setDate(addDays(date, 1))}>›</button
        >
        <button
            class="btn btn-sm"
            onclick={() => setDate(new Date())}
            disabled={isToday(date)}>Today</button
        >
    </div>

    <div class="rows">
        {#await loader}
            <p class="hint pad">Loading tracks…</p>
        {:then tracks}
            {#if tracks.length === 0}
                <p class="hint pad">No tracks recorded for this day.</p>
            {:else}
                <table class="table">
                    <thead>
                        <tr>
                            <th>User</th>
                            <th>Device</th>
                            <th>Start</th>
                            <th>End</th>
                            <th class="actions-col"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each tracks as track (trackKey(track))}
                            <tr
                                class:selected={isSelected(track)}
                                onclick={() => setCurTrack(track)}
                            >
                                <td>{track.user_id}</td>
                                <td>{track.device}</td>
                                <td>{utcToLocalTime(track.ts_start)}</td>
                                <td>{utcToLocalTime(track.ts_end)}</td>
                                <td class="menu-cell">
                                    <button
                                        class="btn btn-sm btn-ghost dots"
                                        aria-label="Export options"
                                        onclick={(e) =>
                                            toggleMenu(trackKey(track), e)}
                                        >⋯</button
                                    >
                                    {#if openMenu === trackKey(track)}
                                        <div class="menu" role="menu">
                                            <a
                                                role="menuitem"
                                                href={gpxDownloadUrl(track)}
                                                onclick={(e) =>
                                                    e.stopPropagation()}
                                                >Download GPX</a
                                            >
                                            <a
                                                role="menuitem"
                                                href={geoJsonDownloadUrl(track)}
                                                onclick={(e) =>
                                                    e.stopPropagation()}
                                                >Download GeoJSON</a
                                            >
                                            <a
                                                role="menuitem"
                                                href={csvDownloadUrl(track)}
                                                onclick={(e) =>
                                                    e.stopPropagation()}
                                                >Download CSV</a
                                            >
                                        </div>
                                    {/if}
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {/if}
        {:catch error}
            <p class="error pad">{error.message}</p>
        {/await}
    </div>
</div>

<style>
    .tracklist {
        display: flex;
        flex-direction: column;
        height: 100%;
        min-height: 0;
    }
    .datebar {
        display: flex;
        align-items: center;
        gap: var(--sp-2);
        flex-wrap: wrap;
        padding-bottom: var(--sp-3);
        border-bottom: 1px solid var(--c-border);
        margin-bottom: var(--sp-2);
    }
    .date {
        flex: 1;
        min-width: 130px;
    }
    .rows {
        flex: 1;
        overflow-y: auto;
        min-height: 0;
    }
    .pad {
        padding: var(--sp-3);
    }
    tbody tr {
        cursor: pointer;
        transition: background-color 0.15s ease;
    }
    tbody tr:hover {
        background: var(--c-surface-2);
    }
    tbody tr.selected {
        background: var(--c-accent-soft);
    }
    .actions-col {
        width: 44px;
    }
    .menu-cell {
        position: relative;
        text-align: right;
        overflow: visible;
    }
    .dots {
        font-size: 1.1rem;
        line-height: 1;
        padding: 4px 10px;
    }
    .menu {
        position: absolute;
        right: 0;
        top: calc(100% + 2px);
        background: var(--c-surface);
        border: 1px solid var(--c-border);
        border-radius: var(--r-md);
        box-shadow: var(--shadow-md);
        z-index: 50;
        min-width: 160px;
        overflow: hidden;
    }
    .menu a {
        display: block;
        padding: 10px 14px;
        color: var(--c-text);
        font-size: var(--fs-sm);
        border-bottom: 1px solid var(--c-border);
    }
    .menu a:last-child {
        border-bottom: none;
    }
    .menu a:hover {
        background: var(--c-surface-2);
        text-decoration: none;
    }
</style>
