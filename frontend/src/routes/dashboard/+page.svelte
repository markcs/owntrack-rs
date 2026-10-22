<script>
    import "../global.css";
    import "../owntracks-theme.css";
    import {
        MapLibre,
        NavigationControl,
        ScaleControl,
        GeoJSONSource,
        FillLayer,
        LineLayer,
        CircleLayer,
    } from "svelte-maplibre-gl";
    import maplibregl from "maplibre-gl";
    const { LngLatBounds } = maplibregl;
    import "maplibre-gl/dist/maplibre-gl.css";
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import { isoDateString, utcOffsetMinutes, addDays } from "../datetime.js";

    let map = $state.raw();

    // --- Date range ---
    let dateFrom = $state(isoDateString(addDays(new Date(), -30)));
    let dateTo = $state(isoDateString(new Date()));

    // Range shortcut presets, matching the official OwnTracks frontend.
    const shortcuts = [
        { label: "Today", days: 0 },
        { label: "7 days", days: 7 },
        { label: "15 days", days: 15 },
        { label: "30 days", days: 30 },
        { label: "3 months", days: 90 },
        { label: "6 months", days: 182 },
        { label: "1 year", days: 365 },
    ];
    function applyShortcut(days) {
        dateTo = isoDateString(new Date());
        dateFrom = isoDateString(addDays(new Date(), -days));
    }

    // --- Devices ---
    let devices = $state([]);
    let selectedDeviceIds = $state(new Set());

    async function loadDevices() {
        const tz = utcOffsetMinutes(new Date());
        try {
            const res = await fetch(
                `${PUBLIC_BASE_URL}/trackinfos?date=${dateTo}&tz=${tz}`,
            );
            const json = await res.json();
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
    let polygons = $state([]);
    let current = $state([]);
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
        if (current.length >= 3) polygons = [...polygons, current];
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

    let drawGeoJSON = $derived.by(() => ({
        type: "FeatureCollection",
        features: polygons.map((ring, i) => ({
            type: "Feature",
            properties: { name: `Area ${i + 1}` },
            geometry: { type: "Polygon", coordinates: [[...ring, ring[0]]] },
        })),
    }));

    let currentGeoJSON = $derived.by(() => ({
        type: "FeatureCollection",
        features: current.map((c) => ({
            type: "Feature",
            properties: {},
            geometry: { type: "Point", coordinates: c },
        })),
    }));

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

    // Fit the map to the drawn polygons.
    function fitView() {
        if (!map || polygons.length === 0) return;
        const bounds = new LngLatBounds();
        for (const ring of polygons) for (const c of ring) bounds.extend(c);
        map.fitBounds(bounds, { padding: 40 });
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
            const res = await fetch(`${PUBLIC_BASE_URL}/metrics/days-in-area`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(body),
            });
            if (!res.ok) throw new Error(await res.text());
            results = (await res.json()).results;
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

    function parseLocalDate(s) {
        const [y, m, d] = s.split("-").map(Number);
        return new Date(y, m - 1, d);
    }

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
        rows.sort(
            (a, b) =>
                a.device.localeCompare(b.device) ||
                a.area.localeCompare(b.area) ||
                a.sortKey.localeCompare(b.sortKey),
        );
        return rows;
    });
</script>

<div class="page">
    <header class="ot-header">
        <span class="header-title">Spatial metrics dashboard</span>
        <div class="ot-nav-item">
            {#if !drawing}
                <button class="button button-outline" onclick={startDrawing}>
                    Draw polygon
                </button>
            {:else}
                <button class="button button-outline" onclick={finishPolygon}>
                    Finish polygon ({current.length} pts)
                </button>
            {/if}
            <button class="button button-outline" onclick={clearPolygons}>
                Clear all
            </button>
            <button
                class="button button-outline"
                onclick={fitView}
                title="Fit the map to the drawn polygons"
            >
                Fit view
            </button>
        </div>
        <div class="ot-nav-item header-right">
            <a class="button button-flat" href="{PUBLIC_BASE_URL}/">
                &larr; Back to map
            </a>
        </div>
    </header>

    <main>
        <div class="layout">
            <div class="map-col">
                <p class="draw-hint">
                    {#if drawing}
                        Click on the map to add points, then "Finish polygon".
                    {:else}
                        {polygons.length} polygon(s) drawn.
                    {/if}
                </p>
                <div class="map-wrap">
                    <MapLibre
                        class="dash-map"
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
                                    "fill-color": "#3f51b5",
                                    "fill-opacity": 0.2,
                                }}
                            />
                            <LineLayer
                                paint={{
                                    "line-color": "#3f51b5",
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
                <section class="ot-card">
                    <h2>Date range</h2>
                    <div class="date-inputs">
                        <label>
                            From
                            <input
                                class="ot-date"
                                type="date"
                                bind:value={dateFrom}
                            />
                        </label>
                        <label>
                            To
                            <input
                                class="ot-date"
                                type="date"
                                bind:value={dateTo}
                            />
                        </label>
                    </div>
                    <div class="shortcuts">
                        {#each shortcuts as s}
                            <button
                                class="button shortcut"
                                onclick={() => applyShortcut(s.days)}
                            >
                                {s.label}
                            </button>
                        {/each}
                    </div>
                </section>

                <section class="ot-card">
                    <h2>Devices</h2>
                    <p class="hint">
                        Leave all unchecked to include every device.
                    </p>
                    {#if devices.length === 0}
                        <p class="hint">No devices found for the end date.</p>
                    {:else}
                        <div class="device-list">
                            {#each devices as d}
                                <label class="ot-check device">
                                    <input
                                        type="checkbox"
                                        checked={selectedDeviceIds.has(
                                            d.device_id,
                                        )}
                                        onchange={() =>
                                            toggleDevice(d.device_id)}
                                    />
                                    {d.label}
                                </label>
                            {/each}
                        </div>
                    {/if}
                </section>

                <section>
                    <button
                        class="button button-primary compute"
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
                    <section class="ot-card">
                        <h2>Results</h2>
                        {#if results.length === 0}
                            <p class="hint">
                                No device had any fix inside the polygon(s) in
                                this range.
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
                                                        ? (r.days / maxDays) *
                                                          100
                                                        : 0}%"
                                                    title={r.dates.join(", ")}
                                                ></div>
                                            </td>
                                        </tr>
                                    {/each}
                                </tbody>
                            </table>

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
    </main>
</div>

<style>
    .page {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
    }
    main {
        flex: 1;
        padding: 20px;
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
        min-width: 300px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }
    .draw-hint {
        color: #666;
        font-size: 0.9em;
        margin-bottom: 8px;
    }
    .map-wrap {
        height: 520px;
        width: 100%;
    }
    :global(.dash-map) {
        height: 100%;
        width: 100%;
        border-radius: 8px;
        overflow: hidden;
    }
    h2 {
        font-size: 1.05em;
        margin-bottom: 10px;
    }
    .date-inputs {
        display: flex;
        gap: 12px;
        flex-wrap: wrap;
        margin-bottom: 12px;
    }
    .date-inputs label {
        display: flex;
        flex-direction: column;
        gap: 4px;
        font-size: 0.9em;
    }
    .shortcuts {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
    }
    .button.shortcut {
        padding: 5px 12px;
        font-size: 0.85em;
        border: 1px solid var(--color-separator);
    }
    .button.shortcut:hover {
        background: #eef0fb;
    }
    .device-list {
        max-height: 200px;
        overflow-y: auto;
    }
    label.device {
        display: flex;
        align-items: center;
        margin-bottom: 4px;
    }
    .compute {
        width: 100%;
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
        border-bottom: 1px solid var(--color-separator);
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
        background: var(--color-primary);
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
