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

    let map = $state.raw();
    let positions_source = $state.raw();
    let hoveredPositionFeat = $state.raw();
    let hoveredPointFeat = $state.raw();
    let lnglat = $state.raw(new maplibregl.LngLat(0, 0));

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
        <GeoJSONSource data={positionsUrl} bind:source={positions_source}>
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
