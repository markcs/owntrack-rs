<script>
    import "./owntracks-theme.css";
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import {
        addDays,
        isoDateString,
        isToday,
        trackQuery,
        utcOffsetMinutes,
        utcToLocalTime,
    } from "./datetime.js";

    let { date, curTrack, setDate, setCurTrack, preselectDeviceId, children } =
        $props();
    let datestr = $derived(isoDateString(date));
    let loader = $derived(load_infos());

    // A device id to auto-select when arriving via a deep link. Consumed once,
    // so manual date changes afterwards fall back to the default selection.
    let pendingPreselect = $state(preselectDeviceId ?? null);

    async function load_infos() {
        const res = await fetch(
            `${PUBLIC_BASE_URL}/trackinfos?date=${datestr}&tz=${utcOffsetMinutes(date)}`,
        );
        const json = await res.json();
        // If we arrived via a deep link naming a device, select that device's
        // track for this day if present.
        if (pendingPreselect != null) {
            const match = json.find((t) => t.device_id === pendingPreselect);
            pendingPreselect = null;
            if (match) {
                setCurTrack(match);
                return json;
            }
        }
        // Select first track, if there is only one for the first day
        if (
            json.length == 1 ||
            (json.length > 1 &&
                isoDateString(json[0].ts_end) !== isoDateString(json[1].ts_end))
        ) {
            setCurTrack(json[0]);
        } else {
            setCurTrack(null);
        }
        return json;
    }

    function checkSelected(track) {
        return (
            curTrack &&
            curTrack.device_id === track.device_id &&
            isoDateString(curTrack.ts_start) === isoDateString(track.ts_start)
        );
    }

    // Clicking a row selects its track; clicking the already-selected row
    // again deselects it, hiding the details panel.
    function toggleTrack(track) {
        if (checkSelected(track)) {
            setCurTrack(null);
        } else {
            setCurTrack(track);
        }
    }

    let track_dropdown = $state(null);

    function toggleDropdown(device_id, event) {
        event.stopPropagation();
        if (track_dropdown === device_id) {
            track_dropdown = null;
        } else {
            track_dropdown = device_id;
            // Position dropdown relative to button
            const button = event.target;
            const rect = button.getBoundingClientRect();
            const dropdown =
                button.parentElement.querySelector(".dropdown-menu");
            if (dropdown) {
                dropdown.style.top = `${rect.bottom + 2}px`;
                dropdown.style.left = `${rect.right}px`;
            }
        }
    }

    function closeDropdown() {
        track_dropdown = null;
    }
</script>

<!-- AppBar-style header (MUI primary app bar). -->
<header class="ot-appbar sidebar-header">
    <span class="ot-title">Owntrack-rs</span>
    <span class="ot-grow"></span>
    <a class="button button-flat" href="{PUBLIC_BASE_URL}/dashboard">
        Dashboard
    </a>
</header>

<!-- Date navigation toolbar. -->
<div class="ot-toolbar sidebar-toolbar ot-paper ot-paper--square">
    <button
        class="ot-icon-button"
        aria-label="Previous day"
        onclick={() => setDate(addDays(date, -1))}
    >
        &#x2039;
    </button>
    <input
        class="ot-date"
        type="date"
        value={datestr}
        onchange={(e) => {
            if (e.target.value) {
                // Parse YYYY-MM-DD as a local calendar date.
                const [y, m, d] = e.target.value.split("-").map(Number);
                setDate(new Date(y, m - 1, d));
            }
        }}
    />
    <button
        class="ot-icon-button"
        aria-label="Next day"
        onclick={() => setDate(addDays(date, 1))}
    >
        &#x203a;
    </button>
    <span class="ot-grow"></span>
    <button
        class="button"
        onclick={() => setDate(new Date())}
        disabled={isToday(date)}
    >
        Today
    </button>
</div>

{@render children?.()}

<!-- Scrollable track list (MUI table inside the Paper surface). -->
<div class="tracks-container ot-paper ot-paper--square">
    {#await loader}
        <p class="tracks-placeholder">Loading track list…</p>
    {:then tracks}
        {#if tracks.length === 0}
            <p class="tracks-placeholder">No tracks for this day.</p>
        {:else}
            <table class="tracks-table">
                <thead>
                    <tr>
                        <th>User</th>
                        <th>Device</th>
                        <th>Start</th>
                        <th>End</th>
                        <th aria-label="Actions"></th>
                    </tr>
                </thead>
                <tbody>
                    {#each tracks as track}
                        <tr
                            class={checkSelected(track) ? "selected" : ""}
                            onclick={() => toggleTrack(track)}
                        >
                            <td>{track.user_id}</td>
                            <td>{track.device}</td>
                            <td>{utcToLocalTime(track.ts_start)}</td>
                            <td>{utcToLocalTime(track.ts_end)}</td>
                            <td class="dropdown-cell">
                                <button
                                    class="ot-icon-button dropdown-button"
                                    aria-label="Track actions"
                                    onclick={(e) =>
                                        toggleDropdown(track.device_id, e)}
                                    >&#x22ee;
                                </button>
                                {#if track_dropdown === track.device_id}
                                    <div class="dropdown-menu ot-paper ot-paper--elevated">
                                        <a
                                            href="{PUBLIC_BASE_URL}/gpxtrack?{trackQuery(
                                                track,
                                            )}"
                                            class="dropdown-item"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                closeDropdown();
                                            }}
                                        >
                                            Download GPX
                                        </a>
                                        <a
                                            href="{PUBLIC_BASE_URL}/track?{trackQuery(
                                                track,
                                            )}"
                                            class="dropdown-item"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                closeDropdown();
                                            }}
                                        >
                                            Download GeoJSON
                                        </a>
                                        <a
                                            href="{PUBLIC_BASE_URL}/csvtrack?{trackQuery(
                                                track,
                                            )}"
                                            class="dropdown-item"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                closeDropdown();
                                            }}
                                        >
                                            Download CSV
                                        </a>
                                    </div>
                                {/if}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    {:catch error}
        <p class="tracks-error">{error.message}</p>
    {/await}
</div>

<style>
    .sidebar-header {
        pointer-events: auto;
        flex: 0 0 auto;
        border-top-left-radius: var(--radius);
        border-top-right-radius: var(--radius);
    }

    .sidebar-toolbar {
        pointer-events: auto;
        flex: 0 0 auto;
        box-shadow: none;
        border-bottom: 1px solid var(--color-divider);
        flex-wrap: nowrap;
    }

    /* Let the date input shrink to fit the 360px sidebar so the year at the
       end of the value is never clipped. */
    .sidebar-toolbar .ot-date {
        flex: 1 1 auto;
        min-width: 0;
        width: 100%;
        box-sizing: border-box;
        font-size: 0.8125rem;
        padding: 7px 8px;
    }

    .sidebar-toolbar .ot-date:focus {
        padding: 6px 7px;
    }

    .sidebar-toolbar .button {
        flex: 0 0 auto;
        min-width: 0;
        padding: 6px 10px;
    }

    .sidebar-toolbar .ot-icon-button {
        flex: 0 0 auto;
    }

    .tracks-container {
        pointer-events: auto;
        flex: 1 1 auto;
        min-height: 0;
        overflow-y: auto;
        overflow-x: hidden;
        box-shadow: none;
        padding: 0;
        border-bottom-left-radius: var(--radius);
        border-bottom-right-radius: var(--radius);
    }

    .tracks-placeholder {
        margin: 0;
        padding: 16px;
        color: var(--color-text-secondary);
        font-size: 0.875rem;
    }

    .tracks-error {
        margin: 0;
        padding: 16px;
        color: #d32f2f;
        font-size: 0.875rem;
    }

    .tracks-table {
        width: 100%;
        table-layout: fixed;
        border-collapse: collapse;
        font-size: 0.8125rem;
    }

    .tracks-table th {
        position: sticky;
        top: 0;
        z-index: 1;
        text-align: left;
        padding: 10px 8px;
        color: var(--color-text-secondary);
        background: var(--color-paper);
        border-bottom: 1px solid var(--color-divider);
        font-weight: 500;
        white-space: nowrap;
    }

    .tracks-table td {
        padding: 10px 8px;
        color: var(--color-text);
        border-bottom: 1px solid var(--color-divider);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    /* Column widths so the Start/End times always fit inside the 360px
       sidebar; User and Device flex and truncate with an ellipsis. */
    .tracks-table th:nth-child(1),
    .tracks-table td:nth-child(1) {
        width: 24%;
    }
    .tracks-table th:nth-child(2),
    .tracks-table td:nth-child(2) {
        width: 26%;
    }
    .tracks-table th:nth-child(3),
    .tracks-table td:nth-child(3),
    .tracks-table th:nth-child(4),
    .tracks-table td:nth-child(4) {
        width: 21%;
    }
    .tracks-table th:nth-child(5),
    .tracks-table td:nth-child(5) {
        width: 32px;
    }

    .tracks-table tbody tr {
        cursor: pointer;
        transition: background-color 0.15s;
    }

    .tracks-table tbody tr:hover {
        background-color: var(--color-action-hover);
    }

    .tracks-table tbody tr.selected {
        background-color: var(--color-action-selected);
    }

    .tracks-table tbody tr.selected:hover {
        background-color: var(--color-action-selected-hover);
    }

    .dropdown-cell {
        position: relative;
        overflow: visible;
        text-align: right;
        padding-left: 0;
        padding-right: 2px;
    }

    .dropdown-button {
        width: 28px;
        height: 28px;
        padding: 2px;
        font-size: 1.1rem;
        line-height: 1;
    }

    .dropdown-menu {
        position: fixed;
        z-index: 9999;
        min-width: 170px;
        padding: 8px 0;
        background: var(--color-paper);
        transform: translate(-100%, 0);
    }

    .dropdown-item {
        display: block;
        padding: 8px 16px;
        color: var(--color-text);
        text-decoration: none;
        font-size: 0.875rem;
        transition: background-color 0.15s;
    }

    .dropdown-item:hover {
        background-color: var(--color-action-hover);
    }
</style>
