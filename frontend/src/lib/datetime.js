// Date and timezone helpers shared across pages.
//
// The backend returns UTC timestamps like '2025-03-01 08:21:16+00' and expects
// a `tz` query parameter that is the client's offset from UTC in minutes.

/** Add (or subtract) whole days to a Date, returning a new Date. */
export function addDays(date, days) {
    const d = new Date(date);
    d.setDate(date.getDate() + days);
    return d;
}

// Normalise an API timestamp ('2025-03-01 08:21:16+00') to an ISO 8601 string
// ('2025-03-01T08:21:16+00:00') so every browser parses it as the same instant.
export function parseUtc(value) {
    if (value instanceof Date) return value;
    return new Date(value.replace(" ", "T").replace(/([+-]\d{2})$/, "$1:00"));
}

/** Local wall-clock time, e.g. '08:21:16'. */
export function utcToLocalTime(value) {
    return parseUtc(value).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
    });
}

/** Local date, e.g. '01.03.2025' in the user's locale. */
export function utcToLocalDate(value) {
    return parseUtc(value).toLocaleDateString();
}

/** Local calendar date as 'YYYY-MM-DD' for a Date or an API timestamp. */
export function isoDateString(value) {
    const d = parseUtc(value);
    const pad = (n) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

// Offset of local time from UTC in minutes (e.g. 660 for Australia/Sydney).
// A Date means that calendar day, so the offset is taken at its local midnight.
// A timestamp string uses its own instant.
export function utcOffsetMinutes(value) {
    let d = parseUtc(value);
    if (value instanceof Date) {
        d = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    }
    return -d.getTimezoneOffset();
}

/** True if the given Date (or timestamp) falls on today's local date. */
export function isToday(value) {
    return isoDateString(value) === isoDateString(new Date());
}

/** Parse a 'YYYY-MM-DD' string as a local calendar date. */
export function parseLocalDate(s) {
    const [y, m, d] = s.split("-").map(Number);
    return new Date(y, m - 1, d);
}
