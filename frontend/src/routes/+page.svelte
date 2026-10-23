<script>
    import "./global.css";
    import Tracklist from "./Tracklist.svelte";
    import Trackinfo from "./Trackinfo.svelte";
    import Tracktitle from "./Tracktitle.svelte";
    import Map from "./Map.svelte";
    import { browser } from "$app/environment";
    import { isoDateString, isToday, utcOffsetMinutes } from "./datetime.js";

    // Optional deep-link params: ?date=YYYY-MM-DD&device=<id> (e.g. from the
    // spatial dashboard results). Parsed on the client since the page is
    // prerendered to static HTML.
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
    let curTrack = $state();
    let trackpoints = $state();
    let positionsSelector = $derived.by(() => {
        // Show positions if date is today
        if (isToday(date)) {
            return `date=${isoDateString(date)}&tz=${utcOffsetMinutes(date)}`;
        } else {
            return null;
        }
    });

    function setDate(newDate) {
        date = newDate;
    }

    function setCurTrack(newTrack) {
        curTrack = newTrack;
    }
    function setTrackpoints(newTrackpoints) {
        trackpoints = newTrackpoints;
    }
</script>

<!-- Traccar-style layout: a full-bleed map with a floating sidebar on top.
     The sidebar holds an AppBar-style header (rendered by Tracklist), the
     scrollable track list (also Tracklist) and, when a track is selected,
     its title and stats. -->
<div class="app-root">
    <div class="map-layer">
        <Map {curTrack} {trackpoints} {positionsSelector} {setCurTrack} />
    </div>

    <aside class="sidebar">
        <Tracklist
            {date}
            {curTrack}
            {setDate}
            {setCurTrack}
            {preselectDeviceId}
        >
            {#if curTrack}
                <div class="sidebar-title ot-paper ot-paper--square">
                    <Tracktitle {curTrack} />
                </div>
            {/if}
        </Tracklist>

        {#if curTrack}
            <div class="sidebar-info ot-paper">
                <Trackinfo {curTrack} {setTrackpoints} />
            </div>
        {/if}
    </aside>
</div>

<style>
    .app-root {
        position: fixed;
        inset: 0;
        overflow: hidden;
    }

    /* Map fills the whole viewport, behind the sidebar. */
    .map-layer {
        position: absolute;
        inset: 0;
        z-index: 1;
    }

    /* Floating sidebar column over the map (MUI MainPage pattern). */
    .sidebar {
        position: absolute;
        top: 0;
        left: 0;
        z-index: 3;
        display: flex;
        flex-direction: column;
        width: var(--drawer-width);
        max-height: calc(100% - 24px);
        margin: 12px;
        gap: 0;
        pointer-events: none;
    }

    .sidebar-title {
        pointer-events: auto;
        padding: 10px 16px;
        box-shadow: none;
        border-bottom: 1px solid var(--color-divider);
        font-size: 0.875rem;
        color: var(--color-text-secondary);
    }

    .sidebar-info {
        pointer-events: auto;
        flex: 0 0 auto;
        padding: 4px 16px;
        margin-top: 12px;
        border-radius: var(--radius);
    }

    /* On small screens the sidebar becomes a bounded panel pinned to the top
       so the map stays visible below it. The track list scrolls inside the
       panel instead of pushing the map off-screen. */
    @media (max-width: 768px) {
        .sidebar {
            width: 100%;
            height: auto;
            max-height: 62vh;
            margin: 0;
            gap: 0;
        }

        .sidebar-info {
            margin-top: 0;
            border-top: 1px solid var(--color-divider);
            border-radius: 0;
            box-shadow: none;
            /* Keep the stats compact so the list keeps most of the room. */
            max-height: 30vh;
            overflow-y: auto;
        }
    }
</style>
