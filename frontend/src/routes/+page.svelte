<script>
    import { browser } from "$app/environment";
    import MapView from "$lib/components/MapView.svelte";
    import TrackList from "$lib/components/TrackList.svelte";
    import TrackStats from "$lib/components/TrackStats.svelte";
    import {
        isoDateString,
        isToday,
        utcOffsetMinutes,
        utcToLocalDate,
        utcToLocalTime,
    } from "$lib/datetime.js";
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
    // Properties of a track point the user clicked on the map, shown in the
    // Track details panel. Cleared whenever the selected track changes.
    let selectedPoint = $state(null);
    // Whether `selectedPoint` was chosen explicitly by the user (map click)
    // rather than defaulted to the latest point. A user pick is kept as-is;
    // an auto-default is replaced as fresher points arrive. Reset whenever the
    // selected track changes or the selection is cleared.
    let userPickedPoint = $state(false);

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
    const setCurTrack = (t) => {
        curTrack = t;
        // A new (or cleared) track invalidates any point selected on the map.
        selectedPoint = null;
        userPickedPoint = false;
    };
    const setTrackpoints = (t) => (trackpoints = t);
    const setSelectedPoint = (p) => {
        selectedPoint = p;
        // A click selects a specific point; a null clears it. Either way this
        // is an explicit user action, so stop auto-defaulting to the latest.
        userPickedPoint = p != null;
        // Make sure the details panel is visible on mobile when a point is
        // picked, so the information the user asked for is not hidden.
        if (p) statsOpen = true;
    };
    const clearSelectedPoint = () => {
        selectedPoint = null;
        userPickedPoint = false;
    };

    // Latest point of the currently loaded track, i.e. the last feature in the
    // GeoJSON FeatureCollection (points are returned in chronological order).
    function latestTrackPoint(tp) {
        const feats = tp && tp.features;
        if (!Array.isArray(feats) || feats.length === 0) return null;
        return feats[feats.length - 1]?.properties ?? null;
    }

    // When a device is selected for today, default the Point details to the
    // latest point unless the user has explicitly picked one. The default
    // follows fresh points as the track grows (live view); once the user picks
    // a point, we leave their choice alone until the track or date changes.
    $effect(() => {
        if (userPickedPoint) return;
        if (!curTrack || !isToday(date)) return;
        const latest = latestTrackPoint(trackpoints);
        if (latest) selectedPoint = latest;
    });

    // --- Resizable side panels -------------------------------------------
    // The sidebar holds two stacked panels (Tracks, Track details). A drag
    // handle between them sets the Tracks panel height; the details panel
    // takes the remaining space. The value is a pixel height, clamped on drag
    // and persisted so the user's preferred split survives reloads.
    const LIST_H_KEY = "viewer.listHeight";
    const MIN_PANEL_H = 120;
    let listHeight = $state(null); // null = use default flex sizing
    let sideEl = $state.raw();
    let dragging = $state(false);

    $effect(() => {
        if (!browser) return;
        const saved = Number(localStorage.getItem(LIST_H_KEY));
        if (Number.isFinite(saved) && saved >= MIN_PANEL_H) listHeight = saved;
    });

    function clampListHeight(px) {
        if (!sideEl) return px;
        // Leave room for the details panel (min height + the handle + gap).
        const max = sideEl.clientHeight - MIN_PANEL_H - 24;
        return Math.max(MIN_PANEL_H, Math.min(px, Math.max(MIN_PANEL_H, max)));
    }

    function onDragMove(ev) {
        if (!dragging || !sideEl) return;
        const clientY = ev.touches ? ev.touches[0].clientY : ev.clientY;
        const top = sideEl.getBoundingClientRect().top;
        listHeight = clampListHeight(clientY - top);
        ev.preventDefault();
    }
    function onDragEnd() {
        if (!dragging) return;
        dragging = false;
        if (browser && listHeight != null)
            localStorage.setItem(LIST_H_KEY, String(Math.round(listHeight)));
        window.removeEventListener("pointermove", onDragMove);
        window.removeEventListener("pointerup", onDragEnd);
    }
    function startDrag(ev) {
        dragging = true;
        window.addEventListener("pointermove", onDragMove);
        window.addEventListener("pointerup", onDragEnd);
        ev.preventDefault();
    }

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
        <MapView
            {curTrack}
            {trackpoints}
            {positionsUrl}
            {setCurTrack}
            {setSelectedPoint}
        />
        {#if latestPosition}
            <div
                class="latest-badge"
                title={`${utcToLocalDate(latestPosition.ts)} ${utcToLocalTime(latestPosition.ts)}`}
            >
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

    <aside class="side" bind:this={sideEl} class:dragging>
        <section
            class="card panel list-panel"
            style={listHeight != null && listOpen
                ? `flex: 0 0 ${listHeight}px`
                : undefined}
        >
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

        <div
            class="resizer"
            class:hidden={!listOpen || !statsOpen}
            role="separator"
            aria-orientation="horizontal"
            aria-label="Resize panels"
            onpointerdown={startDrag}
            ontouchstart={startDrag}
        >
            <span class="grip"></span>
        </div>

        <section class="card panel stats-panel">
            <button
                class="panel-head"
                aria-expanded={statsOpen}
                onclick={() => (statsOpen = !statsOpen)}
            >
                <h2>Track details</h2>
                <span class="caret" class:open={statsOpen}>▾</span>
            </button>
            <div class="panel-body stats-body" class:collapsed={!statsOpen}>
                <TrackStats
                    {curTrack}
                    {setTrackpoints}
                    {selectedPoint}
                    {clearSelectedPoint}
                />
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
        gap: 0;
        min-height: 0;
    }
    .side.dragging {
        cursor: row-resize;
        user-select: none;
    }

    .panel {
        display: flex;
        flex-direction: column;
        min-height: 0;
        padding: 0;
        overflow: hidden;
    }
    /* By default the Tracks panel flexes and the details panel takes what it
       needs; a drag on the resizer overrides this with an explicit height. */
    .list-panel {
        flex: 1 1 auto;
    }
    .stats-panel {
        flex: 1 1 auto;
        margin-top: var(--sp-3);
    }
    .panel-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        background: transparent;
        border: none;
        cursor: pointer;
        padding: var(--sp-3) var(--sp-4);
        font: inherit;
        flex: none;
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
    .stats-body {
        flex: 1;
        overflow-y: auto;
    }

    /* Drag handle between the two stacked panels. */
    .resizer {
        flex: none;
        height: 14px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: row-resize;
        touch-action: none;
    }
    .resizer.hidden {
        display: none;
    }
    .grip {
        width: 44px;
        height: 4px;
        border-radius: var(--r-pill);
        background: var(--c-border-strong);
        transition: background-color 0.15s ease;
    }
    .resizer:hover .grip,
    .side.dragging .grip {
        background: var(--c-accent);
    }

    /* On desktop the panels are always expanded; the collapse toggle is a
       mobile affordance. */
    @media (max-width: 768px) {
        .viewer {
            display: flex;
            flex-direction: column;
            height: auto;
            min-height: calc(100vh - var(--app-bar-h));
            padding: var(--sp-3);
            gap: var(--sp-3);
        }
        .map-pane {
            height: 55vh;
            flex: none;
        }
        .side {
            /* Let the panels size to content on mobile; the whole page
               scrolls rather than each panel individually. */
            display: flex;
            flex-direction: column;
            gap: var(--sp-3);
        }
        .list-panel,
        .stats-panel {
            flex: none;
            /* Ignore any dragged desktop height on mobile. */
            height: auto !important;
            margin-top: 0;
        }
        .list-body {
            max-height: 45vh;
        }
        .stats-body {
            max-height: 55vh;
        }
        /* Resizing is a desktop affordance; the page scrolls on mobile. */
        .resizer {
            display: none;
        }
        .panel-body.collapsed {
            display: none;
        }
    }
</style>
