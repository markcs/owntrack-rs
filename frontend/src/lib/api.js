// Central client for the owntrack-rs backend API.
//
// Every page talks to the backend only through this module, so the set of
// endpoints and their query-string shapes live in one place. The base URL is
// injected at build time (empty in production, where the frontend is served
// from the same origin as the API).

import { PUBLIC_BASE_URL } from "$env/static/public";
import { utcOffsetMinutes } from "./datetime.js";

const BASE = PUBLIC_BASE_URL;

/** Absolute (or origin-relative) URL for a backend path. */
export function apiUrl(path) {
    return `${BASE}${path}`;
}

// --- Track list -----------------------------------------------------------

/**
 * Track infos for a local calendar day.
 * @param {Date} date
 * @returns {Promise<Array<{device_id:number,user_id:string,device:string,tid:string,ts_start:string,ts_end:string}>>}
 */
export async function fetchTrackInfos(date, isoDate, tz) {
    const res = await fetch(
        apiUrl(`/trackinfos?date=${isoDate}&tz=${tz ?? utcOffsetMinutes(date)}`),
    );
    if (!res.ok) throw new Error(`trackinfos failed (${res.status})`);
    return res.json();
}

// --- Track query helpers ---------------------------------------------------

/** Query string identifying one track for the /track* endpoints. */
export function trackQuery(track) {
    const ts = encodeURIComponent(track.ts_start);
    return `device_id=${track.device_id}&ts_start=${ts}&tz=${utcOffsetMinutes(track.ts_start)}`;
}

/** URL for the GeoJSON line of a track (also used directly as a map source). */
export function trackGeoJsonUrl(track) {
    return apiUrl(`/track?${trackQuery(track)}`);
}

export function gpxDownloadUrl(track) {
    return apiUrl(`/gpxtrack?${trackQuery(track)}`);
}
export function geoJsonDownloadUrl(track) {
    return apiUrl(`/track?${trackQuery(track)}`);
}
export function csvDownloadUrl(track) {
    return apiUrl(`/csvtrack?${trackQuery(track)}`);
}

/** GeoJSON track points + stats + bbox for a track. */
export async function fetchTrackpoints(track) {
    const res = await fetch(apiUrl(`/trackpoints?${trackQuery(track)}`));
    if (!res.ok) throw new Error(`trackpoints failed (${res.status})`);
    return res.json();
}

// --- Live positions --------------------------------------------------------

/** URL of the GeoJSON live-positions source for a day (used by MapLibre). */
export function positionsUrl(isoDate, tz) {
    return apiUrl(`/positions?date=${isoDate}&tz=${tz}`);
}

/**
 * The single most recently received device position, or null if none exists.
 * @returns {Promise<{device_id:number,x:number,y:number,ts:string,tid:string}|null>}
 */
export async function fetchLatestPosition() {
    const res = await fetch(apiUrl(`/latest-position`));
    if (!res.ok) throw new Error(`latest-position failed (${res.status})`);
    return res.json();
}

// --- Setup / OTRC ----------------------------------------------------------

/**
 * Fetch the OwnTracks remote configuration.
 * @returns {Promise<{status:"ok"|"invalid"|"error", otrc?:string}>}
 */
export async function fetchOtrc() {
    try {
        const res = await fetch(apiUrl(`/otrc`));
        if (res.status === 403) return { status: "invalid" };
        if (!res.ok) return { status: "error" };
        const json = await res.json();
        return { status: "ok", otrc: btoa(JSON.stringify(json)) };
    } catch {
        return { status: "error" };
    }
}

export function otrcDownloadUrl() {
    return apiUrl(`/otrc`);
}

// --- Spatial metrics -------------------------------------------------------

/**
 * POST the days-in-area query.
 * @returns {Promise<Array<{device_id:number,polygon_index:number,polygon_name?:string,days:number,dates:string[]}>>}
 */
export async function fetchDaysInArea(body) {
    const res = await fetch(apiUrl(`/metrics/days-in-area`), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(await res.text());
    return (await res.json()).results;
}
