//! Live reverse-geocoding against a Nominatim server.
//!
//! Positions are stored immediately on arrival; geocoding happens separately in
//! a background worker so the insert path never blocks on a network call and so
//! we can respect Nominatim's usage policy of at most one request per second.
//!
//! Coordinates are rounded to a fixed number of decimal places and formatted as
//! fixed-precision strings (e.g. `-33.8688`). That rounded pair is the cache
//! key, so nearby fixes share a single lookup. This matches the keying used by
//! the standalone `geocode.py` backfill script, so a cache built by either is
//! usable by the other.

use serde::{Deserialize, Serialize};

/// Nominatim usage policy floor: never faster than one request per second.
const MIN_DELAY_SECS: f64 = 1.0;

/// Configuration for the background geocoder, read from the environment.
#[derive(Clone, Debug)]
pub struct GeocodeConfig {
    /// Whether the background geocoder runs at all.
    pub enabled: bool,
    /// Reverse-geocoding endpoint, e.g.
    /// `https://nominatim.openstreetmap.org/reverse`.
    pub url: String,
    /// `User-Agent` sent with every request (required by Nominatim policy).
    pub user_agent: String,
    /// Decimal places to round coordinates to before caching. 4 d.p. is ~11 m.
    pub precision: u32,
    /// Minimum delay between requests, in seconds. Floored at 1.0.
    pub delay_secs: f64,
    /// How often the worker wakes to look for new work, in seconds.
    pub poll_secs: u64,
    /// How many locations to look up per wake-up before sleeping again.
    pub batch_size: i64,
    /// Entries older than this many days are re-fetched. 0 disables refresh
    /// (entries are kept indefinitely), which is the sensible default because
    /// addresses rarely change and the request budget is scarce.
    pub ttl_days: i64,
}

impl GeocodeConfig {
    /// Build configuration from environment variables, falling back to
    /// conservative defaults that respect Nominatim's public-server policy.
    pub fn from_env() -> Self {
        let enabled = env_bool("GEOCODE_ENABLED", true);
        let url = dotenvy::var("GEOCODE_URL")
            .unwrap_or_else(|_| "https://nominatim.openstreetmap.org/reverse".to_string());
        let user_agent = dotenvy::var("GEOCODE_USER_AGENT").unwrap_or_else(|_| {
            concat!(
                "owntrack-rs/",
                env!("CARGO_PKG_VERSION"),
                " (contact@campbellsmith.me)"
            )
            .to_string()
        });
        let precision = env_parse("GEOCODE_PRECISION", 4);
        let delay_secs = env_parse::<f64>("GEOCODE_DELAY_SECS", 1.0).max(MIN_DELAY_SECS);
        let poll_secs = env_parse("GEOCODE_POLL_SECS", 60);
        let batch_size = env_parse("GEOCODE_BATCH_SIZE", 50);
        let ttl_days = env_parse("GEOCODE_TTL_DAYS", 0);
        GeocodeConfig {
            enabled,
            url,
            user_agent,
            precision,
            delay_secs,
            poll_secs,
            batch_size,
            ttl_days,
        }
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match dotenvy::var(key) {
        Ok(v) => matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"),
        Err(_) => default,
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    dotenvy::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

/// Round a coordinate and format it as a fixed-precision string. This is the
/// cache key; doing it in Rust (rather than in SQL) keeps the logic identical
/// across SQLite and PostgreSQL.
pub fn round_key(value: f64, precision: u32) -> String {
    format!("{value:.*}", precision as usize)
}

/// A reverse-geocoded address, flattened from Nominatim's nested response into
/// the columns stored in `geocode_cache` and surfaced to the UI.
#[derive(Clone, Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct Address {
    pub place_id: Option<i64>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i64>,
    pub category: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub display_name: Option<String>,
    /// Building / POI name, if any.
    pub name: Option<String>,
    pub house_number: Option<String>,
    pub road: Option<String>,
    pub suburb: Option<String>,
    /// city OR town OR village OR hamlet.
    pub city: Option<String>,
    pub state: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}

impl Address {
    /// Whether this address carries any usable component. A response with no
    /// road/city/etc. is treated as "resolved but empty" so the worker does not
    /// retry it forever.
    pub fn has_components(&self) -> bool {
        self.name.is_some()
            || self.house_number.is_some()
            || self.road.is_some()
            || self.suburb.is_some()
            || self.city.is_some()
            || self.state.is_some()
            || self.postcode.is_some()
            || self.country.is_some()
    }

    /// A short human-readable label, best-effort, for map popups and labels.
    pub fn label(&self) -> Option<String> {
        let primary = self
            .name
            .clone()
            .or_else(|| match (&self.house_number, &self.road) {
                (Some(n), Some(r)) => Some(format!("{n} {r}")),
                (None, Some(r)) => Some(r.clone()),
                _ => None,
            });
        let locality = self.city.clone().or_else(|| self.suburb.clone());
        match (primary, locality) {
            (Some(p), Some(l)) if p != l => Some(format!("{p}, {l}")),
            (Some(p), _) => Some(p),
            (None, Some(l)) => Some(l),
            (None, None) => self.display_name.clone(),
        }
    }
}

/// Raw Nominatim reverse-geocoding response (the subset we read).
#[derive(Debug, Deserialize)]
struct NominatimResponse {
    place_id: Option<i64>,
    osm_type: Option<String>,
    osm_id: Option<i64>,
    category: Option<String>,
    #[serde(rename = "type")]
    type_: Option<String>,
    display_name: Option<String>,
    name: Option<String>,
    #[serde(default)]
    address: NominatimAddress,
}

#[derive(Debug, Default, Deserialize)]
struct NominatimAddress {
    building: Option<String>,
    amenity: Option<String>,
    leisure: Option<String>,
    house_number: Option<String>,
    road: Option<String>,
    suburb: Option<String>,
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    hamlet: Option<String>,
    state: Option<String>,
    postcode: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
}

/// Result of a single reverse-geocode lookup: the flattened address and the raw
/// JSON body (kept as a safety net / for future-proofing).
pub struct GeocodeResult {
    pub address: Address,
    pub raw_json: String,
}

/// A thin, reusable Nominatim client. Holds the HTTP client and config; callers
/// are responsible for rate limiting between calls.
#[derive(Clone)]
pub struct Geocoder {
    client: reqwest::Client,
    url: String,
}

impl Geocoder {
    pub fn new(cfg: &GeocodeConfig) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(cfg.user_agent.clone())
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Geocoder {
            client,
            url: cfg.url.clone(),
        })
    }

    /// Reverse-geocode a rounded `(lat, lon)` pair. The strings are the cache
    /// key, so the request uses exactly those values.
    pub async fn reverse(&self, lat_r: &str, lon_r: &str) -> anyhow::Result<GeocodeResult> {
        let resp = self
            .client
            .get(&self.url)
            .query(&[
                ("format", "jsonv2"),
                ("lat", lat_r),
                ("lon", lon_r),
                ("addressdetails", "1"),
                ("layer", "address"),
            ])
            .send()
            .await?
            .error_for_status()?;
        let raw_json = resp.text().await?;
        let parsed: NominatimResponse = serde_json::from_str(&raw_json)?;
        Ok(GeocodeResult {
            address: flatten(parsed),
            raw_json,
        })
    }
}

/// Flatten a Nominatim response into our `Address`, applying the same
/// name/city fallback logic as the Python backfill script.
fn flatten(r: NominatimResponse) -> Address {
    let a = r.address;
    let name = r
        .name
        .or(a.building)
        .or(a.amenity)
        .or(a.leisure)
        .filter(|s| !s.is_empty());
    let city = a.city.or(a.town).or(a.village).or(a.hamlet);
    Address {
        place_id: r.place_id,
        osm_type: r.osm_type,
        osm_id: r.osm_id,
        category: r.category,
        type_: r.type_,
        display_name: r.display_name,
        name,
        house_number: a.house_number,
        road: a.road,
        suburb: a.suburb,
        city,
        state: a.state,
        postcode: a.postcode,
        country: a.country,
        country_code: a.country_code,
    }
}

/// Run the background geocoder loop forever. Spawned once at startup. Does
/// nothing (returns immediately) when disabled.
///
/// Each wake-up it fetches a bounded batch of rounded coordinates that still
/// need an address and resolves them one at a time, never faster than
/// `delay_secs` apart, then sleeps `poll_secs` before looking again. This keeps
/// the request rate within Nominatim's policy regardless of how fast positions
/// arrive, and both backfills history and resolves new fixes through the same
/// path.
pub async fn run_worker(db: crate::db::Db, cfg: GeocodeConfig) {
    if !cfg.enabled {
        log::info!("Geocoder disabled (set GEOCODE_ENABLED=true to enable)");
        return;
    }
    let geocoder = match Geocoder::new(&cfg) {
        Ok(g) => g,
        Err(e) => {
            log::error!("Geocoder init failed, geocoding disabled: {e}");
            return;
        }
    };
    log::info!(
        "Geocoder started: url={} precision={} delay={}s poll={}s batch={}",
        cfg.url,
        cfg.precision,
        cfg.delay_secs,
        cfg.poll_secs,
        cfg.batch_size,
    );

    let delay = std::time::Duration::from_secs_f64(cfg.delay_secs);
    let poll = std::time::Duration::from_secs(cfg.poll_secs.max(1));

    loop {
        match db
            .pending_geocode_coords(cfg.precision, cfg.batch_size, cfg.ttl_days)
            .await
        {
            Ok(pending) if !pending.is_empty() => {
                log::debug!("Geocoder: {} pending location(s)", pending.len());
                for (lat_r, lon_r) in pending {
                    // Rate limit: one request per `delay` at most.
                    tokio::time::sleep(delay).await;
                    match geocoder.reverse(&lat_r, &lon_r).await {
                        Ok(result) => {
                            if let Err(e) = db
                                .upsert_geocode(&lat_r, &lon_r, &result.address, &result.raw_json)
                                .await
                            {
                                log::error!("Geocoder: failed to store {lat_r},{lon_r}: {e}");
                            } else {
                                log::debug!(
                                    "Geocoded {lat_r},{lon_r} -> {}",
                                    result.address.label().unwrap_or_default()
                                );
                            }
                        }
                        Err(e) => {
                            // Transient (network/rate) error: leave uncached so
                            // it is retried on a later pass.
                            log::warn!("Geocoder: reverse lookup {lat_r},{lon_r} failed: {e}");
                        }
                    }
                }
            }
            Ok(_) => {} // nothing pending
            Err(e) => log::error!("Geocoder: failed to query pending coords: {e}"),
        }
        tokio::time::sleep(poll).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_key_formats_fixed_precision() {
        assert_eq!(round_key(-33.868821, 4), "-33.8688");
        assert_eq!(round_key(151.2, 4), "151.2000");
        assert_eq!(round_key(0.0, 3), "0.000");
    }

    #[test]
    fn delay_is_floored_at_one_second() {
        // The floor is applied in from_env; verify the constant and max logic.
        assert_eq!(0.5_f64.max(MIN_DELAY_SECS), 1.0);
        assert_eq!(2.0_f64.max(MIN_DELAY_SECS), 2.0);
    }

    #[test]
    fn flatten_applies_name_and_city_fallbacks() {
        let json = r#"{
            "place_id": 1, "osm_type": "way", "osm_id": 2,
            "category": "highway", "type": "residential",
            "display_name": "Foo Street, Bar Town, Country",
            "address": {
                "road": "Foo Street",
                "village": "Bar Town",
                "country": "Country",
                "country_code": "cc"
            }
        }"#;
        let parsed: NominatimResponse = serde_json::from_str(json).unwrap();
        let addr = flatten(parsed);
        assert_eq!(addr.road.as_deref(), Some("Foo Street"));
        // city falls back to village
        assert_eq!(addr.city.as_deref(), Some("Bar Town"));
        assert!(addr.has_components());
        assert_eq!(addr.label().as_deref(), Some("Foo Street, Bar Town"));
    }

    #[test]
    fn empty_address_has_no_components() {
        let addr = Address::default();
        assert!(!addr.has_components());
        assert_eq!(addr.label(), None);
    }

    #[test]
    fn name_prefers_building_over_empty() {
        let json = r#"{"address": {"building": "Town Hall", "city": "Metropolis"}}"#;
        let parsed: NominatimResponse = serde_json::from_str(json).unwrap();
        let addr = flatten(parsed);
        assert_eq!(addr.name.as_deref(), Some("Town Hall"));
        assert_eq!(addr.label().as_deref(), Some("Town Hall, Metropolis"));
    }
}
