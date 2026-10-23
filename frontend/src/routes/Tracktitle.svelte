<script>
    import { utcToLocalDate, utcToLocalTime } from "./datetime.js";

    let { curTrack } = $props();
</script>

{#if curTrack}
    <div class="track-title">
        <img src="/favicon.svg" alt="" width="16" height="16" />
        <span>
            Track from {utcToLocalDate(curTrack.ts_end)}
            {utcToLocalTime(curTrack.ts_start)} – {utcToLocalTime(
                curTrack.ts_end,
            )}
        </span>
    </div>
{/if}

<style>
    .track-title {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 0.9375rem;
        font-weight: 500;
        color: var(--color-text);
    }

    .track-title img {
        flex: 0 0 auto;
    }
</style>
