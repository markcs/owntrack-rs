<script>
    import { browser } from "$app/environment";
    import MapView from "$lib/components/MapView.svelte";
    import TrackList from "$lib/components/TrackList.svelte";
    import TrackStats from "$lib/components/TrackStats.svelte";
    import { isoDateString, isToday, utcOffsetMinutes } from "$lib/datetime.js";
    import {
        positionsUrl as positionsUrlFn,
        fetchLatestPosition,
    } from "$lib/api.js";

    // Optional deep-link params: ?date=YYYY-MM-DD&device=<id>, e.g. from the
    // dashboard results. Parsed on the client since the page is prerendered.
    function initialState() {
        if (browser) {
            const q = new URLSearchParams(window.location.search);
            const dateParam = q.get("date");
            const deviceParam = q.get("device");
            let date = new Date();
            if (dateParam && /^\d{4}-\d{2}-\d{2}$/.test(dateParam)) {
                const [y, m, d] = dateParam.split("-").map(Number);
                date = new Date(y, m - 1, d);
            }
            const deviceId =
                deviceParam != null && deviceParam !== ""
                    ? Number(deviceParam)
                    : null;
            return { date, deviceId: Number.isNaN(deviceId) ? null : deviceId };
        }
        return { date: new Date(), deviceId: null };
    }

    const init = initialState();
    let date = $state(init.date);
    let preselectDeviceId = $state(init.deviceId);
    let curTrack = $state(null);
    let trackpoints = $state(null);

    // Live positions only make sense for today.
    let positionsUrl = $derived(
        isToday(date)
            ? positionsUrlFn(isoDateString(date), utcOffsetMinutes(date))
            : null,
    );

    // Mobile panel collapse state.
    let listOpen = $state(true);
    let statsOpen = $state(true);

    const setDate = (d) => (date = d);
    const setCurTrack = (t) => (curTrack = t);
    const setTrackpoints = (t) => (trackpoints = t);

    // The most recently received position, shown as a "last seen" label with
    // its resolved address. Refreshed periodically so an address that the
    // background geocoder resolves after the fix arrives appears without a
    // manual reload. Failures are non-fatal (the label simply stays hidden).
    let latestPosition = $state(null);
    async function refreshLatest() {
        try {
            const pos = await fetchLatestPosition();
            if (pos && typeof pos.x === "number") latestPosition = pos;
        } catch (e) {
            console.error("Failed to load latest position", e);
        }
    }
    $effect(() => {
        if (!browser) return;
        refreshLatest();
        const timer = setInterval(refreshLatest, 20_000);
        return () => clearInterval(timer);
    });
</script>

<div class="viewer">
    <section class="map-pane card">
        <MapView {curTrack} {trackpoints} {positionsUrl} {setCurTrack} />
        {#if latestPosition}
            <div class="latest-badge" title={latestPosition.ts}>
                <span class="latest-label">Last seen</span>
                {#if latestPosition.address}
                    <span class="latest-addr">{latestPosition.address}</span>
                {:else}
                    <span class="latest-addr muted"
                        >{latestPosition.y.toFixed(4)}, {latestPosition.x.toFixed(
                            4,
                        )}</span
                    >
                {/if}
            </div>
        {/if}
    </section>

    <aside class="side">
        <section class="card panel">
            <button
                class="panel-head"
                aria-expanded={listOpen}
                onclick={() => (listOpen = !listOpen)}
            >
                <h2>Tracks</h2>
                <span class="caret" class:open={listOpen}>▾</span>
            </button>
            <div class="panel-body list-body" class:collapsed={!listOpen}>
                <TrackList
                    {date}
                    {curTrack}
                    {setDate}
                    {setCurTrack}
                    {preselectDeviceId}
                />
            </div>
        </section>

        <section class="card panel">
            <button
                class="panel-head"
                aria-expanded={statsOpen}
                onclick={() => (statsOpen = !statsOpen)}
            >
                <h2>Track details</h2>
                <span class="caret" class:open={statsOpen}>▾</span>
            </button>
            <div class="panel-body" class:collapsed={!statsOpen}>
                <TrackStats {curTrack} {setTrackpoints} />
            </div>
        </section>
    </aside>
</div>

<style>
    .viewer {
        display: grid;
        grid-template-columns: 1fr 380px;
        grid-template-rows: 1fr auto;
        gap: var(--sp-4);
        padding: var(--sp-4);
        height: calc(100vh - var(--app-bar-h));
    }
    .map-pane {
        grid-row: 1 / span 2;
        padding: var(--sp-2);
        min-height: 0;
        position: relative;
    }
    .latest-badge {
        position: absolute;
        top: calc(var(--sp-2) + var(--sp-2));
        left: calc(var(--sp-2) + var(--sp-2));
        z-index: 2;
        display: flex;
        flex-direction: column;
        gap: 2px;
        max-width: 60%;
        padding: var(--sp-2) var(--sp-3);
        background: rgba(255, 255, 255, 0.92);
        border-radius: var(--radius, 8px);
        box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
        pointer-events: none;
        font-size: var(--fs-sm, 0.85rem);
    }
    .latest-label {
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--c-text-muted);
    }
    .latest-addr {
        font-weight: 600;
        color: var(--c-text, #222);
    }
    .latest-addr.muted {
        font-weight: 400;
        color: var(--c-text-muted);
    }
    .side {
        grid-column: 2;
        grid-row: 1 / span 2;
        display: flex;
        flex-direction: column;
        gap: var(--sp-4);
        min-height: 0;
    }

    .panel {
        display: flex;
        flex-direction: column;
        min-height: 0;
        padding: 0;
    }
    .panel:first-child {
        flex: 1;
    }
    .panel-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        background: transparent;
        border: none;
        cursor: pointer;
        padding: var(--sp-4);
        font: inherit;
    }
    .panel-head h2 {
        font-size: var(--fs-lg);
    }
    .caret {
        transition: transform 0.2s ease;
        color: var(--c-text-muted);
    }
    .caret.open {
        transform: rotate(180deg);
    }
    .panel-body {
        padding: 0 var(--sp-4) var(--sp-4);
        min-height: 0;
    }
    .list-body {
        flex: 1;
        display: flex;
        flex-direction: column;
    }

    /* On desktop the panels are always expanded; the collapse toggle is a
       mobile affordance. */
    @media (max-width: 768px) {
        .viewer {
            display: flex;
            flex-direction: column;
            height: auto;
            min-height: calc(100vh - var(--app-bar-h));
        }
        .map-pane {
            height: 55vh;
            flex: none;
        }
        .panel:first-child {
            flex: none;
        }
        .list-body {
            max-height: 50vh;
        }
        .panel-body.collapsed {
            display: none;
        }
    }
</style>
