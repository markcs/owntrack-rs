<script>
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
    import "maplibre-gl/dist/maplibre-gl.css";
    import { browser } from "$app/environment";
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import {
        isoDateString,
        utcOffsetMinutes,
        addDays,
        parseLocalDate,
    } from "$lib/datetime.js";
    import { fetchTrackInfos, fetchDaysInArea } from "$lib/api.js";

    const { LngLatBounds } = maplibregl;

    let map = $state.raw();

    // --- Date range ---
    let dateFrom = $state(isoDateString(addDays(new Date(), -30)));
    let dateTo = $state(isoDateString(new Date()));
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
        try {
            const json = await fetchTrackInfos(
                parseLocalDate(dateTo),
                dateTo,
                utcOffsetMinutes(new Date()),
            );
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
        next.has(id) ? next.delete(id) : next.add(id);
        selectedDeviceIds = next;
    }

    // --- Mode & weekday filter ---
    let mode = $state("inside");
    let selectedWeekdays = $state(new Set([0, 1, 2, 3, 4, 5, 6]));
    function toggleWeekday(i) {
        const next = new Set(selectedWeekdays);
        next.has(i) ? next.delete(i) : next.add(i);
        selectedWeekdays = next;
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
        const wd = [...selectedWeekdays].sort((a, b) => a - b);
        const weekdays = wd.length === 7 ? [] : wd;
        try {
            results = await fetchDaysInArea({
                date_from: dateFrom,
                date_to: dateTo,
                tz: utcOffsetMinutes(new Date()),
                device_ids: [...selectedDeviceIds],
                polygons: requestFeatureCollection(),
                mode,
                weekdays,
            });
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
    let computeLabel = $derived(
        mode === "outside" ? "Compute days NOT in area" : "Compute days in area",
    );

    const MONTHS = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    const WEEKDAYS = [
        "Sunday", "Monday", "Tuesday", "Wednesday",
        "Thursday", "Friday", "Saturday",
    ];
    const WEEKDAYS_SHORT = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    function areaLabel(r) {
        if (mode === "outside") return "Outside all areas";
        return r.polygon_name ?? `Area ${r.polygon_index + 1}`;
    }

    let dateRows = $derived.by(() => {
        if (!results) return [];
        const rows = [];
        for (const r of results) {
            for (const ds of r.dates) {
                const d = parseLocalDate(ds);
                rows.push({
                    device_id: r.device_id,
                    device: deviceLabel(r.device_id),
                    area: areaLabel(r),
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

    function trackUrl(row) {
        const q = new URLSearchParams({
            date: row.date,
            device: String(row.device_id),
        });
        return `${PUBLIC_BASE_URL}/?${q.toString()}`;
    }

    // --- CSV export ---
    function csvCell(value) {
        const s = value == null ? "" : String(value);
        return `"${s.replace(/"/g, '""')}"`;
    }
    function buildCsv() {
        const header = ["Device", "Area", "Month", "Date", "Day", "Location"];
        const lines = [header.map(csvCell).join(",")];
        for (const row of dateRows) {
            const rel = trackUrl(row);
            const url = browser
                ? new URL(rel, window.location.origin).href
                : rel;
            lines.push(
                [row.device, row.area, row.month, row.date, row.weekday, url]
                    .map(csvCell)
                    .join(","),
            );
        }
        return "\ufeff" + lines.join("\r\n") + "\r\n";
    }
    function exportCsv() {
        if (!dateRows.length) return;
        const blob = new Blob([buildCsv()], {
            type: "text/csv;charset=utf-8;",
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        const kind = mode === "outside" ? "days-not-in-area" : "days-in-area";
        a.download = `${kind}_${dateFrom}_${dateTo}.csv`;
        document.body.appendChild(a);
        a.click();
        a.remove();
        URL.revokeObjectURL(url);
    }
</script>

<div class="dashboard">
    <div class="map-col card">
        <div class="map-toolbar">
            <div class="draw-tools">
                {#if !drawing}
                    <button class="btn btn-sm btn-primary" onclick={startDrawing}
                        >Draw polygon</button
                    >
                {:else}
                    <button class="btn btn-sm btn-primary" onclick={finishPolygon}
                        >Finish ({current.length} pts)</button
                    >
                {/if}
                <button class="btn btn-sm" onclick={clearPolygons}>Clear</button>
                <button class="btn btn-sm" onclick={fitView}>Fit view</button>
            </div>
            <span class="hint">
                {#if drawing}
                    Click the map to add points, then Finish.
                {:else}
                    {polygons.length} polygon(s) drawn.
                {/if}
            </span>
        </div>
        <div class="map-wrap">
            <MapLibre
                class="maparea"
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
                        paint={{ "fill-color": "#0e9488", "fill-opacity": 0.2 }}
                    />
                    <LineLayer
                        paint={{ "line-color": "#0e9488", "line-width": 2 }}
                    />
                </GeoJSONSource>
                <GeoJSONSource data={currentGeoJSON}>
                    <CircleLayer
                        paint={{ "circle-color": "#f59e0b", "circle-radius": 4 }}
                    />
                </GeoJSONSource>
            </MapLibre>
        </div>
    </div>

    <div class="controls-col">
        <section class="card">
            <h2>Date range</h2>
            <div class="date-inputs">
                <label>
                    From
                    <input class="input" type="date" bind:value={dateFrom} />
                </label>
                <label>
                    To
                    <input class="input" type="date" bind:value={dateTo} />
                </label>
            </div>
            <div class="shortcuts">
                {#each shortcuts as s}
                    <button class="chip chip-btn" onclick={() => applyShortcut(s.days)}
                        >{s.label}</button
                    >
                {/each}
            </div>
        </section>

        <section class="card">
            <h2>Devices</h2>
            <p class="hint">Leave all unchecked to include every device.</p>
            {#if devices.length === 0}
                <p class="hint">No devices found for the end date.</p>
            {:else}
                <div class="device-list">
                    {#each devices as d}
                        <label class="check device">
                            <input
                                type="checkbox"
                                checked={selectedDeviceIds.has(d.device_id)}
                                onchange={() => toggleDevice(d.device_id)}
                            />
                            {d.label}
                        </label>
                    {/each}
                </div>
            {/if}
        </section>

        <section class="card">
            <h2>Calculation</h2>
            <div class="mode-group">
                <label class="check">
                    <input
                        type="radio"
                        name="mode"
                        checked={mode === "inside"}
                        onchange={() => (mode = "inside")}
                    />
                    Days inside the polygon(s)
                </label>
                <label class="check">
                    <input
                        type="radio"
                        name="mode"
                        checked={mode === "outside"}
                        onchange={() => (mode = "outside")}
                    />
                    Days NOT in the polygon(s)
                </label>
            </div>
            <p class="hint">
                {#if mode === "inside"}
                    Counts days a device had at least one fix inside a polygon.
                {:else}
                    Counts days a device was active but recorded no fix inside
                    any polygon.
                {/if}
            </p>

            <h3 class="sub">Days of the week</h3>
            <p class="hint">Leave all selected to include every day.</p>
            <div class="weekday-group">
                {#each WEEKDAYS_SHORT as label, i}
                    <label class="check">
                        <input
                            type="checkbox"
                            checked={selectedWeekdays.has(i)}
                            onchange={() => toggleWeekday(i)}
                        />
                        {label}
                    </label>
                {/each}
            </div>
        </section>

        <button
            class="btn btn-primary compute"
            onclick={compute}
            disabled={loading}
        >
            {loading ? "Computing…" : computeLabel}
        </button>
        {#if error}
            <p class="error">{error}</p>
        {/if}

        {#if results}
            <section class="card">
                <div class="results-head">
                    <h2>Results</h2>
                    {#if results.length > 0}
                        <button class="btn btn-sm" onclick={exportCsv}
                            >Export CSV</button
                        >
                    {/if}
                </div>
                {#if results.length === 0}
                    <p class="hint">
                        {#if mode === "outside"}
                            No device had an active day outside the polygon(s)
                            in this range.
                        {:else}
                            No device had any fix inside the polygon(s) in this
                            range.
                        {/if}
                    </p>
                {:else}
                    <div class="table-scroll">
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>Device</th>
                                    <th>Area</th>
                                    <th class="num">Days</th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each results as r}
                                    <tr>
                                        <td>{deviceLabel(r.device_id)}</td>
                                        <td>{areaLabel(r)}</td>
                                        <td class="num days">{r.days}</td>
                                        <td class="bar-cell">
                                            <div
                                                class="bar"
                                                style="width: {maxDays
                                                    ? (r.days / maxDays) * 100
                                                    : 0}%"
                                                title={r.dates.join(', ')}
                                            ></div>
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>

                    <h3 class="sub">Days detail</h3>
                    <div class="table-scroll detail">
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>Device</th>
                                    <th>Area</th>
                                    <th>Month</th>
                                    <th>Date</th>
                                    <th>Day</th>
                                    <th>Location</th>
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
                                        <td>
                                            <a
                                                href={trackUrl(row)}
                                                target="_blank"
                                                rel="noopener"
                                                >View track</a
                                            >
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    </div>
                {/if}
            </section>
        {/if}
    </div>
</div>

<style>
    .dashboard {
        display: grid;
        grid-template-columns: 2fr 1fr;
        gap: var(--sp-4);
        padding: var(--sp-4);
        align-items: start;
    }
    .map-col {
        padding: var(--sp-3);
        position: sticky;
        top: calc(var(--app-bar-h) + var(--sp-4));
    }
    .map-toolbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: var(--sp-3);
        flex-wrap: wrap;
        margin-bottom: var(--sp-3);
    }
    .draw-tools {
        display: flex;
        gap: var(--sp-2);
        flex-wrap: wrap;
    }
    .map-wrap {
        height: 60vh;
        min-height: 380px;
    }
    .controls-col {
        display: flex;
        flex-direction: column;
        gap: var(--sp-4);
        min-width: 0;
    }
    .date-inputs {
        display: flex;
        gap: var(--sp-3);
        flex-wrap: wrap;
        margin-bottom: var(--sp-3);
    }
    .date-inputs label {
        display: flex;
        flex-direction: column;
        gap: var(--sp-1);
        font-size: var(--fs-sm);
        color: var(--c-text-muted);
    }
    .shortcuts {
        display: flex;
        flex-wrap: wrap;
        gap: var(--sp-2);
    }
    .chip-btn {
        cursor: pointer;
        font: inherit;
        font-size: var(--fs-xs);
    }
    .chip-btn:hover {
        background: var(--c-accent-soft);
        color: var(--c-accent-strong);
    }
    .device-list {
        max-height: 200px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: var(--sp-2);
    }
    .mode-group {
        display: flex;
        flex-direction: column;
        gap: var(--sp-2);
        margin-bottom: var(--sp-2);
    }
    .sub {
        font-size: var(--fs-md);
        margin: var(--sp-4) 0 var(--sp-2);
    }
    .weekday-group {
        display: flex;
        flex-wrap: wrap;
        gap: var(--sp-3);
        margin-top: var(--sp-2);
    }
    .compute {
        width: 100%;
    }
    .results-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: var(--sp-3);
    }
    .results-head h2 {
        margin-bottom: 0;
    }
    .table-scroll {
        overflow-x: auto;
    }
    .table-scroll.detail {
        max-height: 340px;
        overflow-y: auto;
    }
    .num {
        text-align: right;
    }
    .days {
        font-weight: 600;
    }
    .bar-cell {
        width: 120px;
    }
    .bar {
        height: 10px;
        background: var(--c-accent);
        border-radius: var(--r-sm);
        min-width: 2px;
    }

    @media (max-width: 900px) {
        .dashboard {
            grid-template-columns: 1fr;
        }
        .map-col {
            position: static;
        }
        .map-wrap {
            height: 50vh;
        }
    }
</style>
