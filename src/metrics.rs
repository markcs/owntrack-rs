//! Spatial metrics over recorded positions.
//!
//! Currently computes the number of distinct local calendar days a device had
//! at least one position fix inside a user-defined polygon.

use crate::db::{local_date_from_epoch, local_day_window, Db, RangePoint};
use chrono::{Datelike, NaiveDate};
use geo::{Contains, MultiPolygon, Point};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Default accuracy threshold in meters. Fixes less accurate than this are
/// ignored, matching the filter used for track exports.
const MAX_ACCURACY: i32 = 200;

/// Which days to count.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Days the device had at least one fix inside a polygon (per polygon).
    #[default]
    Inside,
    /// Days the device was active (had any accurate fix) but had no fix inside
    /// any polygon. Reported per device, aggregated across all polygons.
    Outside,
}

/// Request body for `POST /metrics/days-in-area`.
#[derive(Deserialize, Debug)]
pub struct DaysInAreaRequest {
    /// Inclusive local start date, `YYYY-MM-DD`.
    pub date_from: String,
    /// Inclusive local end date, `YYYY-MM-DD`.
    pub date_to: String,
    /// Client's offset from UTC in minutes (e.g. 660 for Australia/Sydney).
    #[serde(default)]
    pub tz: i32,
    /// Restrict to these device ids. Empty / omitted means all devices.
    #[serde(default)]
    pub device_ids: Vec<i32>,
    /// Polygons to test against, as a GeoJSON FeatureCollection.
    pub polygons: geojson::FeatureCollection,
    /// Discard fixes less accurate than this many meters. Defaults to 200.
    pub max_accuracy: Option<i32>,
    /// Count days inside a polygon, or days active-but-outside. Default inside.
    #[serde(default)]
    pub mode: Mode,
    /// Restrict counting to these weekdays, `0 = Sunday .. 6 = Saturday`
    /// (matching the browser's `Date.getDay()`). Empty / omitted means all days.
    #[serde(default)]
    pub weekdays: Vec<u8>,
}

/// Map a `NaiveDate` to the browser's weekday numbering: 0 = Sunday .. 6 = Saturday.
fn weekday_index(date: NaiveDate) -> u8 {
    // chrono: Monday = 0 .. Sunday = 6 (num_days_from_monday).
    // Browser: Sunday = 0 .. Saturday = 6.
    date.weekday().num_days_from_sunday() as u8
}

/// Whether `date` is allowed by the weekday filter. An empty filter allows all.
fn weekday_allowed(date: NaiveDate, weekdays: &[u8]) -> bool {
    weekdays.is_empty() || weekdays.contains(&weekday_index(date))
}

/// Day count for one device inside one polygon.
#[derive(Serialize, Debug, PartialEq)]
pub struct AreaDays {
    pub device_id: i32,
    /// Index of the polygon in the request FeatureCollection.
    pub polygon_index: usize,
    /// Polygon label, taken from the feature's `name` property if present.
    pub polygon_name: Option<String>,
    /// Number of distinct local calendar days with a fix inside the polygon.
    pub days: usize,
    /// The distinct local days, `YYYY-MM-DD`, ascending.
    pub dates: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct DaysInAreaResponse {
    pub results: Vec<AreaDays>,
}

/// A polygon to test against, with its label and geometry.
struct Area {
    name: Option<String>,
    geom: MultiPolygon<f64>,
}

/// Parse a GeoJSON FeatureCollection into testable polygon areas.
/// Non-polygon geometries are skipped.
fn parse_areas(fc: &geojson::FeatureCollection) -> anyhow::Result<Vec<Area>> {
    let mut areas = Vec::new();
    for feature in &fc.features {
        let Some(geometry) = &feature.geometry else {
            continue;
        };
        let name = feature
            .property("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let geom: geo::Geometry<f64> = geometry.try_into()?;
        let multi = match geom {
            geo::Geometry::Polygon(p) => MultiPolygon::new(vec![p]),
            geo::Geometry::MultiPolygon(mp) => mp,
            _ => continue, // ignore points, lines, etc.
        };
        areas.push(Area { name, geom: multi });
    }
    Ok(areas)
}

/// Parse `YYYY-MM-DD`.
fn parse_date(s: &str) -> anyhow::Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%F").map_err(|_| anyhow::anyhow!("date must be YYYY-MM-DD"))
}

/// Core aggregation for inside mode: for each (device, polygon) pair, the set
/// of distinct local days (passing the weekday filter) on which that device
/// had at least one accurate fix inside the polygon.
fn aggregate_inside(
    points: &[RangePoint],
    areas: &[Area],
    tz: i32,
    max_accuracy: i32,
    weekdays: &[u8],
) -> Vec<AreaDays> {
    // (polygon_index, device_id) -> set of local days
    let mut buckets: BTreeMap<(usize, i32), BTreeSet<NaiveDate>> = BTreeMap::new();

    for pt in points {
        if pt.accuracy.unwrap_or(0) >= max_accuracy {
            continue;
        }
        let Ok(day) = local_date_from_epoch(pt.ts, tz) else {
            continue;
        };
        if !weekday_allowed(day, weekdays) {
            continue;
        }
        // Coordinate order is (x, y) = (lon, lat).
        let point = Point::new(pt.x, pt.y);
        for (idx, area) in areas.iter().enumerate() {
            if area.geom.contains(&point) {
                buckets.entry((idx, pt.device_id)).or_default().insert(day);
            }
        }
    }

    buckets
        .into_iter()
        .map(|((polygon_index, device_id), days)| AreaDays {
            device_id,
            polygon_index,
            polygon_name: areas[polygon_index].name.clone(),
            days: days.len(),
            dates: days.iter().map(|d| d.to_string()).collect(),
        })
        .collect()
}

/// Core aggregation for outside mode: for each device, the set of distinct
/// local days (passing the weekday filter) on which the device had at least one
/// accurate fix but none of them fell inside any polygon.
fn aggregate_outside(
    points: &[RangePoint],
    areas: &[Area],
    tz: i32,
    max_accuracy: i32,
    weekdays: &[u8],
) -> Vec<AreaDays> {
    // device_id -> all active days, and the subset of days with an inside fix.
    let mut active: BTreeMap<i32, BTreeSet<NaiveDate>> = BTreeMap::new();
    let mut inside: BTreeMap<i32, BTreeSet<NaiveDate>> = BTreeMap::new();

    for pt in points {
        if pt.accuracy.unwrap_or(0) >= max_accuracy {
            continue;
        }
        let Ok(day) = local_date_from_epoch(pt.ts, tz) else {
            continue;
        };
        if !weekday_allowed(day, weekdays) {
            continue;
        }
        active.entry(pt.device_id).or_default().insert(day);

        let point = Point::new(pt.x, pt.y);
        if areas.iter().any(|a| a.geom.contains(&point)) {
            inside.entry(pt.device_id).or_default().insert(day);
        }
    }

    active
        .into_iter()
        .filter_map(|(device_id, active_days)| {
            let inside_days = inside.get(&device_id);
            let outside_days: BTreeSet<NaiveDate> = active_days
                .into_iter()
                .filter(|d| inside_days.map(|s| !s.contains(d)).unwrap_or(true))
                .collect();
            if outside_days.is_empty() {
                return None;
            }
            Some(AreaDays {
                device_id,
                // Outside is aggregated across all polygons; index 0 / no name
                // signals "outside all areas" to the client.
                polygon_index: 0,
                polygon_name: None,
                days: outside_days.len(),
                dates: outside_days.iter().map(|d| d.to_string()).collect(),
            })
        })
        .collect()
}

/// Compute distinct days per device per polygon for the given request.
pub async fn days_in_area(db: &Db, req: &DaysInAreaRequest) -> anyhow::Result<DaysInAreaResponse> {
    let from = parse_date(&req.date_from)?;
    let to = parse_date(&req.date_to)?;
    if to < from {
        anyhow::bail!("date_to must not be before date_from");
    }
    // Window covers [from 00:00 local, to+1 day 00:00 local).
    let (start, _) = local_day_window(from, req.tz);
    let (_, end) = local_day_window(to, req.tz);

    let areas = parse_areas(&req.polygons)?;
    if areas.is_empty() {
        return Ok(DaysInAreaResponse { results: vec![] });
    }

    let device_ids = if req.device_ids.is_empty() {
        None
    } else {
        Some(req.device_ids.as_slice())
    };
    let points = db.query_positions_in_range(start, end, device_ids).await?;

    let max_accuracy = req.max_accuracy.unwrap_or(MAX_ACCURACY);
    let results = match req.mode {
        Mode::Inside => aggregate_inside(&points, &areas, req.tz, max_accuracy, &req.weekdays),
        Mode::Outside => aggregate_outside(&points, &areas, req.tz, max_accuracy, &req.weekdays),
    };
    Ok(DaysInAreaResponse { results })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(rfc3339: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .timestamp()
    }

    /// A square polygon around (lon 151.2, lat -33.8), roughly 0.1 degrees wide.
    fn square_fc() -> geojson::FeatureCollection {
        let json = serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": { "name": "Area 1" },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [151.15, -33.85],
                        [151.25, -33.85],
                        [151.25, -33.75],
                        [151.15, -33.75],
                        [151.15, -33.85]
                    ]]
                }
            }]
        });
        serde_json::from_value(json).unwrap()
    }

    fn pt(device_id: i32, lon: f64, lat: f64, ts: i64, accuracy: Option<i32>) -> RangePoint {
        RangePoint {
            device_id,
            x: lon,
            y: lat,
            ts,
            accuracy,
        }
    }

    #[test]
    fn counts_distinct_days_inside_polygon() {
        let areas = parse_areas(&square_fc()).unwrap();
        // Two fixes inside on 22 Oct (Sydney), one inside on 23 Oct, one outside.
        let points = vec![
            // 03:00 and 05:00 UTC 22 Oct -> same Sydney day (22 Oct)
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
            pt(1, 151.21, -33.81, epoch("2026-10-22T05:00:00Z"), None),
            // 03:00 UTC 23 Oct -> 23 Oct Sydney
            pt(1, 151.20, -33.80, epoch("2026-10-23T03:00:00Z"), None),
            // far away, same day
            pt(1, 0.0, 0.0, epoch("2026-10-22T06:00:00Z"), None),
        ];
        let res = aggregate_inside(&points, &areas, 660, 200, &[]);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].device_id, 1);
        assert_eq!(res[0].polygon_name.as_deref(), Some("Area 1"));
        assert_eq!(res[0].days, 2);
        assert_eq!(res[0].dates, vec!["2026-10-22", "2026-10-23"]);
    }

    #[test]
    fn distinct_days_use_local_timezone() {
        let areas = parse_areas(&square_fc()).unwrap();
        // 20:00 UTC 21 Oct and 03:00 UTC 22 Oct are both 22 Oct in Sydney (+11),
        // so this is a single day, not two.
        let points = vec![
            pt(1, 151.20, -33.80, epoch("2026-10-21T20:00:00Z"), None),
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
        ];
        let res = aggregate_inside(&points, &areas, 660, 200, &[]);
        assert_eq!(res[0].days, 1);
        assert_eq!(res[0].dates, vec!["2026-10-22"]);

        // In UTC they fall on two different days.
        let res_utc = aggregate_inside(&points, &areas, 0, 200, &[]);
        assert_eq!(res_utc[0].days, 2);
    }

    #[test]
    fn inaccurate_fixes_are_ignored() {
        let areas = parse_areas(&square_fc()).unwrap();
        let points = vec![pt(
            1,
            151.20,
            -33.80,
            epoch("2026-10-22T03:00:00Z"),
            Some(500),
        )];
        let res = aggregate_inside(&points, &areas, 660, 200, &[]);
        assert!(res.is_empty());
    }

    #[test]
    fn separate_devices_counted_separately() {
        let areas = parse_areas(&square_fc()).unwrap();
        let points = vec![
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
            pt(2, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
        ];
        let res = aggregate_inside(&points, &areas, 660, 200, &[]);
        assert_eq!(res.len(), 2);
        assert!(res.iter().all(|r| r.days == 1));
    }

    // Browser weekday numbering: Sunday = 0 .. Saturday = 6.
    // 22 Oct 2026 is a Thursday (4); 23 Oct 2026 is a Friday (5).

    #[test]
    fn weekday_index_matches_browser_numbering() {
        let thu = NaiveDate::parse_from_str("2026-10-22", "%F").unwrap();
        let fri = NaiveDate::parse_from_str("2026-10-23", "%F").unwrap();
        let sun = NaiveDate::parse_from_str("2026-10-25", "%F").unwrap();
        assert_eq!(weekday_index(thu), 4);
        assert_eq!(weekday_index(fri), 5);
        assert_eq!(weekday_index(sun), 0);
    }

    #[test]
    fn weekday_filter_restricts_inside_days() {
        let areas = parse_areas(&square_fc()).unwrap();
        let points = vec![
            // Thursday 22 Oct (Sydney)
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
            // Friday 23 Oct (Sydney)
            pt(1, 151.20, -33.80, epoch("2026-10-23T03:00:00Z"), None),
        ];
        // Only Fridays (5).
        let res = aggregate_inside(&points, &areas, 660, 200, &[5]);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].days, 1);
        assert_eq!(res[0].dates, vec!["2026-10-23"]);

        // Thursday and Friday both allowed -> both days.
        let res = aggregate_inside(&points, &areas, 660, 200, &[4, 5]);
        assert_eq!(res[0].days, 2);

        // Empty filter allows every day.
        let res = aggregate_inside(&points, &areas, 660, 200, &[]);
        assert_eq!(res[0].days, 2);
    }

    #[test]
    fn outside_counts_active_days_without_inside_fix() {
        let areas = parse_areas(&square_fc()).unwrap();
        let points = vec![
            // 22 Oct: a fix inside the polygon -> NOT an outside day.
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
            // 23 Oct: active but only a fix far away -> an outside day.
            pt(1, 0.0, 0.0, epoch("2026-10-23T03:00:00Z"), None),
            // 24 Oct: active, far away -> outside day.
            pt(1, 10.0, 10.0, epoch("2026-10-24T03:00:00Z"), None),
        ];
        let res = aggregate_outside(&points, &areas, 660, 200, &[]);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].device_id, 1);
        assert_eq!(res[0].polygon_name, None);
        assert_eq!(res[0].days, 2);
        assert_eq!(res[0].dates, vec!["2026-10-23", "2026-10-24"]);
    }

    #[test]
    fn outside_excludes_days_with_any_inside_fix() {
        let areas = parse_areas(&square_fc()).unwrap();
        // Same day has one inside and one outside fix -> the day is inside,
        // so it does not count as an outside day.
        let points = vec![
            pt(1, 0.0, 0.0, epoch("2026-10-22T01:00:00Z"), None),
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
        ];
        let res = aggregate_outside(&points, &areas, 660, 200, &[]);
        assert!(res.is_empty());
    }

    #[test]
    fn outside_respects_weekday_filter() {
        let areas = parse_areas(&square_fc()).unwrap();
        let points = vec![
            // Thursday 22 Oct, outside the polygon.
            pt(1, 0.0, 0.0, epoch("2026-10-22T03:00:00Z"), None),
            // Friday 23 Oct, outside the polygon.
            pt(1, 0.0, 0.0, epoch("2026-10-23T03:00:00Z"), None),
        ];
        // Only count Fridays.
        let res = aggregate_outside(&points, &areas, 660, 200, &[5]);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].days, 1);
        assert_eq!(res[0].dates, vec!["2026-10-23"]);
    }

    #[test]
    fn mode_defaults_to_inside() {
        let req: DaysInAreaRequest = serde_json::from_value(serde_json::json!({
            "date_from": "2026-10-22",
            "date_to": "2026-10-23",
            "polygons": { "type": "FeatureCollection", "features": [] },
        }))
        .unwrap();
        assert_eq!(req.mode, Mode::Inside);
        assert!(req.weekdays.is_empty());
    }

    #[test]
    fn mode_deserializes_outside() {
        let req: DaysInAreaRequest = serde_json::from_value(serde_json::json!({
            "date_from": "2026-10-22",
            "date_to": "2026-10-23",
            "mode": "outside",
            "weekdays": [1, 2, 3, 4, 5],
            "polygons": { "type": "FeatureCollection", "features": [] },
        }))
        .unwrap();
        assert_eq!(req.mode, Mode::Outside);
        assert_eq!(req.weekdays, vec![1, 2, 3, 4, 5]);
    }
}
