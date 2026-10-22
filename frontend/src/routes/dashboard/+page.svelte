<script>
    import "../global.css";
    import {
        MapLibre,
        NavigationControl,
        ScaleControl,
        GeoJSONSource,
        FillLayer,
        LineLayer,
        CircleLayer,
    } from "svelte-maplibre-gl";
    import "maplibre-gl/dist/maplibre-gl.css";
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import { isoDateString, utcOffsetMinutes, addDays } from "../datetime.js";

    let map = $state.raw();

    // --- Date range ---
    let dateFrom = $state(isoDateString(addDays(new Date(), -30)));
    let dateTo = $state(isoDateString(new Date()));

    // --- Devices ---
    // Populated from /trackinfos for the end date; the user may also just
    // leave all selected to query every device.
    let devices = $state([]);
    let selectedDeviceIds = $state(new Set());

    async function loadDevices() {
        const tz = utcOffsetMinutes(new Date());
        try {
            const res = await fetch(
                `${PUBLIC_BASE_URL}/trackinfos?date=${dateTo}&tz=${tz}`,
            );
            const json = await res.json();
            // De-duplicate by device_id.
            const seen = new Map();
            for (const t of json) {
                if (!seen.has(t.device_id)) {
                    seen.set(t.device_id, {
                        device_id: t.device_id,
                        label: `${t.user_id}/${t.device}`,
                    });
                }
            }
            devices = [...seen.values()];
        } catch (e) {
            console.error("Failed to load devices", e);
            devices = [];
        }
    }

    $effect(() => {
        // reload device list whenever the end date changes
        dateTo;
        loadDevices();
    });

    function toggleDevice(id) {
        const next = new Set(selectedDeviceIds);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        selectedDeviceIds = next;
    }

    // --- Polygon drawing ---
    // Click to add vertices; "Finish polygon" closes the current ring and
    // starts a new one. Polygons are stored as arrays of [lon, lat].
    let polygons = $state([]); // finished polygons: [[lon,lat], ...]
    let current = $state([]); // vertices of the polygon being drawn
    let drawing = $state(false);

    function onMapClick(ev) {
        if (!drawing) return;
        current = [...current, [ev.lngLat.lng, ev.lngLat.lat]];
    }

    function startDrawing() {
        drawing = true;
        current = [];
    }

    function finishPolygon() {
        if (current.length >= 3) {
            polygons = [...polygons, current];
        }
        current = [];
        drawing = false;
    }

    function clearPolygons() {
        polygons = [];
        current = [];
        drawing = false;
        results = null;
        error = null;
    }

    // GeoJSON for rendering drawn polygons (closed rings) + current vertices.
    let drawGeoJSON = $derived.by(() => {
        const features = [];
        polygons.forEach((ring, i) => {
            features.push({
                type: "Feature",
                properties: { name: `Area ${i + 1}` },
                geometry: {
                    type: "Polygon",
                    coordinates: [[...ring, ring[0]]],
                },
            });
        });
        return { type: "FeatureCollection", features };
    });

    let currentGeoJSON = $derived.by(() => ({
        type: "FeatureCollection",
        features: current.map((c) => ({
            type: "Feature",
            properties: {},
            geometry: { type: "Point", coordinates: c },
        })),
    }));

    // FeatureCollection sent to the backend (named "Area N").
    function requestFeatureCollection() {
        return {
            type: "FeatureCollection",
            features: polygons.map((ring, i) => ({
                type: "Feature",
                properties: { name: `Area ${i + 1}` },
                geometry: {
                    type: "Polygon",
                    coordinates: [[...ring, ring[0]]],
                },
            })),
        };
    }

    // --- Compute ---
    let loading = $state(false);
    let results = $state(null);
    let error = $state(null);

    async function compute() {
        error = null;
        results = null;
        if (polygons.length === 0) {
            error = "Draw at least one polygon first.";
            return;
        }
        loading = true;
        const body = {
            date_from: dateFrom,
            date_to: dateTo,
            tz: utcOffsetMinutes(new Date()),
            device_ids: [...selectedDeviceIds],
            polygons: requestFeatureCollection(),
        };
        try {
            const res = await fetch(
                `${PUBLIC_BASE_URL}/metrics/days-in-area`,
                {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify(body),
                },
            );
            if (!res.ok) {
                throw new Error(await res.text());
            }
            const json = await res.json();
            results = json.results;
        } catch (e) {
            error = e.message || String(e);
        } finally {
            loading = false;
        }
    }

    function deviceLabel(id) {
        const d = devices.find((d) => d.device_id === id);
        return d ? d.label : `device ${id}`;
    }

    // Max day count, for scaling the bar chart.
    let maxDays = $derived(
        results && results.length
            ? Math.max(...results.map((r) => r.days))
            : 0,
    );

    const MONTHS = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    const WEEKDAYS = [
        "Sunday", "Monday", "Tuesday", "Wednesday",
        "Thursday", "Friday", "Saturday",
    ];

    // Parse a YYYY-MM-DD string as a local calendar date (no TZ shift).
    function parseLocalDate(s) {
        const [y, m, d] = s.split("-").map(Number);
        return new Date(y, m - 1, d);
    }

    // Flatten results into one row per individual date, for the detail table.
    let dateRows = $derived.by(() => {
        if (!results) return [];
        const rows = [];
        for (const r of results) {
            for (const ds of r.dates) {
                const d = parseLocalDate(ds);
                rows.push({
                    device: deviceLabel(r.device_id),
                    area: r.polygon_name ?? `Area ${r.polygon_index + 1}`,
                    month: `${MONTHS[d.getMonth()]} ${d.getFullYear()}`,
                    date: ds,
                    weekday: WEEKDAYS[d.getDay()],
                    sortKey: ds,
                });
            }
        }
        // Sort by device, then area, then date ascending.
        rows.sort(
            (a, b) =>
                a.device.localeCompare(b.device) ||
                a.area.localeCompare(b.area) ||
                a.sortKey.localeCompare(b.sortKey),
        );
        return rows;
    });
</script>

<div class="dashboard">
    <header>
        <h1>Spatial metrics dashboard</h1>
        <a class="back" href="{PUBLIC_BASE_URL}/">&larr; Back to map</a>
    </header>

    <div class="layout">
        <div class="map-col">
            <div class="draw-toolbar">
                {#if !drawing}
                    <button onclick={startDrawing}>Draw polygon</button>
                {:else}
                    <button onclick={finishPolygon}
                        >Finish polygon ({current.length} pts)</button
                    >
                {/if}
                <button onclick={clearPolygons}>Clear all</button>
                <span class="hint">
                    {#if drawing}
                        Click on the map to add points, then "Finish polygon".
                    {:else}
                        {polygons.length} polygon(s) drawn.
                    {/if}
                </span>
            </div>
            <div class="map-wrap">
                <MapLibre
                    class="map"
                    bind:map
                    autoloadGlobalCss={false}
                    style="https://basemaps.cartocdn.com/gl/voyager-gl-style/style.json"
                    center={[0, 20]}
                    zoom={1}
                    onclick={onMapClick}
                >
                    <NavigationControl />
                    <ScaleControl />
                    <GeoJSONSource data={drawGeoJSON}>
                        <FillLayer
                            paint={{
                                "fill-color": "#3388ff",
                                "fill-opacity": 0.25,
                            }}
                        />
                        <LineLayer
                            paint={{
                                "line-color": "#3388ff",
                                "line-width": 2,
                            }}
                        />
                    </GeoJSONSource>
                    <GeoJSONSource data={currentGeoJSON}>
                        <CircleLayer
                            paint={{
                                "circle-color": "#ff6600",
                                "circle-radius": 4,
                            }}
                        />
                    </GeoJSONSource>
                </MapLibre>
            </div>
        </div>

        <div class="controls-col">
            <section>
                <h2>Date range</h2>
                <label>
                    From
                    <input type="date" bind:value={dateFrom} />
                </label>
                <label>
                    To
                    <input type="date" bind:value={dateTo} />
                </label>
            </section>

            <section>
                <h2>Devices</h2>
                <p class="hint">
                    Leave all unchecked to include every device.
                </p>
                {#if devices.length === 0}
                    <p class="hint">No devices found for the end date.</p>
                {:else}
                    <div class="device-list">
                        {#each devices as d}
                            <label class="device">
                                <input
                                    type="checkbox"
                                    checked={selectedDeviceIds.has(
                                        d.device_id,
                                    )}
                                    onchange={() => toggleDevice(d.device_id)}
                                />
                                {d.label}
                            </label>
                        {/each}
                    </div>
                {/if}
            </section>

            <section>
                <button
                    class="compute"
                    onclick={compute}
                    disabled={loading}
                >
                    {loading ? "Computing…" : "Compute days in area"}
                </button>
                {#if error}
                    <p class="error">{error}</p>
                {/if}
            </section>

            {#if results}
                <section>
                    <h2>Results</h2>
                    {#if results.length === 0}
                        <p class="hint">
                            No device had any fix inside the polygon(s) in this
                            range.
                        </p>
                    {:else}
                        <table class="results">
                            <thead>
                                <tr>
                                    <th>Device</th>
                                    <th>Area</th>
                                    <th>Days</th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each results as r}
                                    <tr>
                                        <td>{deviceLabel(r.device_id)}</td>
                                        <td
                                            >{r.polygon_name ??
                                                `Area ${r.polygon_index + 1}`}</td
                                        >
                                        <td class="days">{r.days}</td>
                                        <td class="bar-cell">
                                            <div
                                                class="bar"
                                                style="width: {maxDays
                                                    ? (r.days / maxDays) * 100
                                                    : 0}%"
                                                title={r.dates.join(", ")}
                                            ></div>
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                        <p class="hint">Hover a bar to see the exact days.</p>

                        <h2 class="detail-heading">Days detail</h2>
                        <table class="results detail">
                            <thead>
                                <tr>
                                    <th>Device</th>
                                    <th>Area</th>
                                    <th>Month</th>
                                    <th>Date</th>
                                    <th>Day</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each dateRows as row}
                                    <tr>
                                        <td>{row.device}</td>
                                        <td>{row.area}</td>
                                        <td>{row.month}</td>
                                        <td>{row.date}</td>
                                        <td>{row.weekday}</td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    {/if}
                </section>
            {/if}
        </div>
    </div>
</div>

<style>
    .dashboard {
        padding: 16px;
    }
    header {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        margin-bottom: 12px;
    }
    h1 {
        font-size: 1.4em;
    }
    .back {
        text-decoration: none;
        color: #3388ff;
    }
    .layout {
        display: flex;
        gap: 20px;
        flex-wrap: wrap;
    }
    .map-col {
        flex: 2;
        min-width: 320px;
    }
    .controls-col {
        flex: 1;
        min-width: 280px;
    }
    .draw-toolbar {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 8px;
        flex-wrap: wrap;
    }
    .map-wrap {
        height: 480px;
        width: 100%;
    }
    :global(.map) {
        height: 100%;
        width: 100%;
    }
    section {
        margin-bottom: 18px;
    }
    h2 {
        font-size: 1.05em;
        margin-bottom: 6px;
    }
    label {
        display: block;
        margin-bottom: 6px;
    }
    .device-list {
        max-height: 180px;
        overflow-y: auto;
        border: 1px solid #ddd;
        border-radius: 4px;
        padding: 6px;
    }
    label.device {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-bottom: 2px;
    }
    button {
        cursor: pointer;
        padding: 6px 12px;
        border: 1px solid #ccc;
        border-radius: 4px;
        background: #f6f6f6;
    }
    button:hover {
        background: #ececec;
    }
    .compute {
        background: #3388ff;
        color: white;
        border-color: #2277ee;
        font-weight: bold;
    }
    .compute:disabled {
        opacity: 0.6;
        cursor: default;
    }
    .hint {
        color: #666;
        font-size: 0.85em;
    }
    .error {
        color: #c00;
        margin-top: 8px;
    }
    .results {
        width: 100%;
        border-collapse: collapse;
    }
    .results th,
    .results td {
        text-align: left;
        padding: 6px 8px;
        border-bottom: 1px solid #eee;
    }
    .results .days {
        font-weight: bold;
        text-align: right;
    }
    .bar-cell {
        width: 120px;
    }
    .bar {
        height: 12px;
        background: #3388ff;
        border-radius: 2px;
        min-width: 2px;
    }
    .detail-heading {
        margin-top: 16px;
    }
    .results.detail {
        display: block;
        max-height: 320px;
        overflow-y: auto;
    }
</style>
