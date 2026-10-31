<script>
    import {
        MapLibre,
        NavigationControl,
        ScaleControl,
        GeoJSONSource,
        FeatureState,
        LineLayer,
        CircleLayer,
        SymbolLayer,
        Popup,
        Image,
    } from "svelte-maplibre-gl";
    import maplibregl from "maplibre-gl";
    import "maplibre-gl/dist/maplibre-gl.css";
    import { trackGeoJsonUrl } from "$lib/api.js";

    const { LngLatBounds } = maplibregl;

    let { curTrack, trackpoints, positionsUrl, setCurTrack } = $props();

    // How often the live positions source is re-fetched, in milliseconds.
    const REFRESH_MS = 20_000;

    let map = $state.raw();
    let positions_source = $state.raw();
    let hoveredPositionFeat = $state.raw();
    let hoveredPointFeat = $state.raw();
    let lnglat = $state.raw(new maplibregl.LngLat(0, 0));

    // Whether the map should keep the selected device centred as live
    // positions refresh. Engaged whenever a device is (re)selected and the
    // live view is active; paused when the user pans or zooms by hand.
    let following = $state(false);

    // Zoom to the selected track's points.
    $effect(() => {
        if (trackpoints && map && trackpoints.bbox) {
            map.fitBounds(new LngLatBounds(trackpoints.bbox), { padding: 40 });
        }
    });

    // Zoom to the live positions when they load.
    $effect(() => {
        if (positions_source && map) {
            positions_source.getData().then((data) => {
                if (data && data.bbox) {
                    map.fitBounds(new LngLatBounds(data.bbox), { padding: 40 });
                }
            });
        }
    });

    // Re-engage following whenever a device is (re)selected while the live
    // view is active. A static historical track (no positionsUrl) has nothing
    // to follow, so leave following off then.
    $effect(() => {
        if (curTrack && positionsUrl) {
            following = true;
        } else {
            following = false;
        }
    });

    // Pan the map to the selected device's latest live position. Pan only
    // (easeTo with centre), so the user's chosen zoom level is preserved.
    function followSelected(data) {
        if (!following || !map || !curTrack || !data) return;
        const feat = data.features?.find(
            (f) => f.properties?.device_id === curTrack.device_id,
        );
        const coords = feat?.geometry?.coordinates;
        if (coords) {
            map.easeTo({ center: coords, duration: 1000 });
        }
    }

    // A manual pan or zoom pauses following until a device is (re)selected.
    // Programmatic moves (easeTo/fitBounds) carry no originalEvent, so they
    // are ignored here.
    function onUserMove(ev) {
        if (ev && ev.originalEvent) following = false;
    }

    // Attach manual-interaction listeners once the map exists. Also follow the
    // selected device whenever the live-positions source finishes (re)loading,
    // so a freshly fetched position is centred as soon as it arrives.
    $effect(() => {
        if (!map) return;
        const onSourceData = (ev) => {
            if (ev.sourceId === "positions" && ev.isSourceLoaded) {
                positions_source?.getData().then(followSelected);
            }
        };
        map.on("dragstart", onUserMove);
        map.on("zoomstart", onUserMove);
        map.on("rotatestart", onUserMove);
        map.on("sourcedata", onSourceData);
        return () => {
            map.off("dragstart", onUserMove);
            map.off("zoomstart", onUserMove);
            map.off("rotatestart", onUserMove);
            map.off("sourcedata", onSourceData);
        };
    });

    // Periodically re-fetch the live positions and push the fresh GeoJSON into
    // the source so markers move in the background. We fetch ourselves with a
    // cache-busting query param and `no-store`, because calling setData(url)
    // with a constant URL is subject to HTTP caching and can silently return
    // stale data. Following happens via the sourcedata handler once the new
    // data has loaded.
    async function refreshPositions() {
        if (!positions_source || !positionsUrl) return;
        try {
            const sep = positionsUrl.includes("?") ? "&" : "?";
            const res = await fetch(`${positionsUrl}${sep}_=${Date.now()}`, {
                cache: "no-store",
            });
            if (!res.ok) return;
            const data = await res.json();
            positions_source.setData(data);
        } catch (e) {
            console.error("positions refresh failed", e);
        }
    }

    $effect(() => {
        // Arm the timer whenever the live view is active. We depend only on
        // positionsUrl (a plain prop, reliably reactive); the source readiness
        // is checked inside refreshPositions, so a late-binding source never
        // leaves us without a running timer.
        if (!positionsUrl) return;
        const timer = setInterval(refreshPositions, REFRESH_MS);
        return () => clearInterval(timer);
    });

    // When a device is (re)selected, centre on it immediately using whatever
    // live data is already loaded. Depends on curTrack so switching device
    // while following re-centres too.
    $effect(() => {
        // Reference curTrack so this effect re-runs when the selection changes.
        void curTrack;
        if (following && positions_source) {
            positions_source.getData().then(followSelected);
        }
    });

    function positionToTrack(pos) {
        return { ...pos, ts_start: pos.time, ts_end: pos.time };
    }

    function propsToHtml(obj) {
        let str = "";
        for (const p in obj) {
            if (Object.prototype.hasOwnProperty.call(obj, p)) {
                str += `${p}: ${obj[p]}<br/>`;
            }
        }
        return str;
    }

    // Pulsing dot drawn on a canvas for live positions with movement.
    const pulseSize = 170;
    const pulsingDot = {
        width: pulseSize,
        height: pulseSize,
        data: new Uint8Array(pulseSize * pulseSize * 4),
        onAdd() {
            const canvas = document.createElement("canvas");
            canvas.width = this.width;
            canvas.height = this.height;
            this.context = canvas.getContext("2d");
        },
        render() {
            const t = (performance.now() % 1000) / 1000;
            const radius = (pulseSize / 2) * 0.3;
            const outerRadius = (pulseSize / 2) * 0.7 * t + radius;
            const ctx = this.context;
            ctx.clearRect(0, 0, this.width, this.height);
            ctx.beginPath();
            ctx.arc(this.width / 2, this.height / 2, outerRadius, 0, Math.PI * 2);
            ctx.fillStyle = `rgba(245,158,11, ${1 - t})`;
            ctx.fill();
            this.data = ctx.getImageData(0, 0, this.width, this.height).data;
            return true;
        },
    };
</script>

<MapLibre
    class="maparea"
    bind:map
    autoloadGlobalCss={false}
    style="https://basemaps.cartocdn.com/gl/voyager-gl-style/style.json"
    maxZoom={17}
>
    <NavigationControl />
    <ScaleControl />
    <Image id="pulsing-dot" image={pulsingDot} options={{ pixelRatio: 2 }} />

    {#if trackpoints}
        <GeoJSONSource data={trackpoints}>
            <CircleLayer
                paint={{ "circle-color": "#0e9488", "circle-radius": 5 }}
                onmousemove={(ev) => {
                    hoveredPointFeat = ev.features[0];
                    lnglat = ev.lngLat;
                }}
                onmouseout={() => (hoveredPointFeat = undefined)}
                minzoom={11}
            />
            {#if hoveredPointFeat}
                <FeatureState id={hoveredPointFeat.id} state={{ hover: true }} />
                <Popup {lnglat} closeButton={false}>
                    {@html propsToHtml(hoveredPointFeat.properties)}
                </Popup>
            {/if}
        </GeoJSONSource>
    {/if}

    {#if curTrack}
        <GeoJSONSource data={trackGeoJsonUrl(curTrack)}>
            <LineLayer
                paint={{ "line-color": "#0e9488", "line-width": 4 }}
            />
        </GeoJSONSource>
    {/if}

    {#if positionsUrl}
        <GeoJSONSource
            id="positions"
            data={positionsUrl}
            bind:source={positions_source}
        >
            <SymbolLayer
                layout={{
                    "icon-image": [
                        "case",
                        [">", ["get", "speed"], 0],
                        "pulsing-dot",
                        "",
                    ],
                    "icon-allow-overlap": true,
                }}
            />
            <CircleLayer
                paint={{
                    "circle-color": [
                        "case",
                        ["boolean", ["feature-state", "hover"], false],
                        "#0b7a70",
                        "#0e9488",
                    ],
                    "circle-radius": 18,
                }}
                onmousemove={(ev) => (hoveredPositionFeat = ev.features[0])}
                onmouseout={() => (hoveredPositionFeat = undefined)}
                onclick={(ev) =>
                    setCurTrack(positionToTrack(ev.features[0].properties))}
            />
            <SymbolLayer
                layout={{ "text-field": ["get", "tid"], "text-size": 14 }}
                paint={{ "text-color": "#ffffff" }}
            />
            {#if hoveredPositionFeat}
                <FeatureState
                    id={hoveredPositionFeat.id}
                    state={{ hover: true }}
                />
            {/if}
        </GeoJSONSource>
    {/if}
</MapLibre>
