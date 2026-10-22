//! Spatial metrics over recorded positions.
//!
//! Currently computes the number of distinct local calendar days a device had
//! at least one position fix inside a user-defined polygon.

use crate::db::{local_date_from_epoch, local_day_window, Db, RangePoint};
use chrono::NaiveDate;
use geo::{Contains, MultiPolygon, Point};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Default accuracy threshold in meters. Fixes less accurate than this are
/// ignored, matching the filter used for track exports.
const MAX_ACCURACY: i32 = 200;

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

/// Core aggregation: for each (device, polygon) pair, the set of distinct local
/// days on which that device had at least one accurate fix inside the polygon.
fn aggregate(points: &[RangePoint], areas: &[Area], tz: i32, max_accuracy: i32) -> Vec<AreaDays> {
    // (polygon_index, device_id) -> set of local days
    let mut buckets: std::collections::BTreeMap<(usize, i32), BTreeSet<NaiveDate>> =
        std::collections::BTreeMap::new();

    for pt in points {
        if pt.accuracy.unwrap_or(0) >= max_accuracy {
            continue;
        }
        // Coordinate order is (x, y) = (lon, lat).
        let point = Point::new(pt.x, pt.y);
        let Ok(day) = local_date_from_epoch(pt.ts, tz) else {
            continue;
        };
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
    let results = aggregate(&points, &areas, req.tz, max_accuracy);
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
        let res = aggregate(&points, &areas, 660, 200);
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
        let res = aggregate(&points, &areas, 660, 200);
        assert_eq!(res[0].days, 1);
        assert_eq!(res[0].dates, vec!["2026-10-22"]);

        // In UTC they fall on two different days.
        let res_utc = aggregate(&points, &areas, 0, 200);
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
        let res = aggregate(&points, &areas, 660, 200);
        assert!(res.is_empty());
    }

    #[test]
    fn separate_devices_counted_separately() {
        let areas = parse_areas(&square_fc()).unwrap();
        let points = vec![
            pt(1, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
            pt(2, 151.20, -33.80, epoch("2026-10-22T03:00:00Z"), None),
        ];
        let res = aggregate(&points, &areas, 660, 200);
        assert_eq!(res.len(), 2);
        assert!(res.iter().all(|r| r.days == 1));
    }
}
