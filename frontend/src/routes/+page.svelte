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

<div class="container">
    <div class="top-section">
        <div class="left-column">
            <div class="map-section">
                <div class="map-title">
                    <Tracktitle {curTrack} />
                </div>
                <div class="map-container">
                    <Map
                        {curTrack}
                        {trackpoints}
                        {positionsSelector}
                        {setCurTrack}
                    />
                </div>
            </div>

            <div class="trackinfo-container">
                <Trackinfo {curTrack} {setTrackpoints} />
            </div>
        </div>

        <div class="tracklist-container">
            <Tracklist {date} {curTrack} {setDate} {setCurTrack} {preselectDeviceId} />
        </div>
    </div>
</div>

<style>
    * {
        box-sizing: border-box;
        margin: 0;
        padding: 0;
    }

    .container {
        width: 100%;
        margin: 0 auto;
        display: flex;
        flex-direction: column;
    }

    .top-section {
        display: flex;
        flex-wrap: wrap;
        width: 100%;
    }

    .left-column {
        flex: 1;
        min-width: 300px;
        padding-right: 20px;
        display: flex;
        flex-direction: column;
    }

    .map-title {
        margin-bottom: 6px;
    }

    .map-section {
        width: 100%;
        position: relative;
        margin-bottom: 20px;
    }

    .map-container {
        width: 100%;
    }

    .trackinfo-container {
        width: 100%;
        margin-bottom: 20px;
    }

    .tracklist-container {
        flex: 1;
        min-width: 300px;
        max-height: calc(
            100vh - 40px
        ); /* Limit to screen height minus some padding */
        overflow-y: auto; /* Make it scrollable */
        padding-right: 10px; /* Add some padding to accommodate scrollbar */
    }

    @media (max-width: 768px) {
        .container {
            display: flex;
            flex-direction: column;
        }

        .top-section {
            flex-direction: column;
        }

        .left-column {
            width: 100%;
            padding-right: 0;
            margin-bottom: 20px;
        }

        .map-section {
            width: 100%;
            margin-bottom: 20px;
        }

        .tracklist-container {
            width: 100%;
            margin-bottom: 20px;
            max-height: 400px; /* Limit height on mobile */
            overflow-y: auto;
        }
    }
</style>
