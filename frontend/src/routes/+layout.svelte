<script>
    import "$lib/styles/fonts.css";
    import "$lib/styles/app.css";
    import { PUBLIC_BASE_URL } from "$env/static/public";
    import { page } from "$app/stores";

    let { children } = $props();

    const links = [
        { href: "/", label: "Map", match: (p) => p === "/" },
        {
            href: "/dashboard",
            label: "Dashboard",
            match: (p) => p.startsWith("/dashboard"),
        },
        {
            href: "/setup",
            label: "Setup",
            match: (p) => p.startsWith("/setup"),
        },
    ];

    let menuOpen = $state(false);
    let pathname = $derived($page.url.pathname);

    function hrefOf(link) {
        return `${PUBLIC_BASE_URL}${link.href}`;
    }
    function close() {
        menuOpen = false;
    }
</script>

<div class="shell">
    <header class="appbar">
        <a class="brand" href="{PUBLIC_BASE_URL}/" onclick={close}>
            <img src="/favicon.svg" alt="" width="22" height="22" />
            <span>Owntrack-rs</span>
        </a>

        <nav class="nav-desktop" aria-label="Primary">
            {#each links as link}
                <a
                    class="navlink"
                    class:active={link.match(pathname)}
                    aria-current={link.match(pathname) ? "page" : undefined}
                    href={hrefOf(link)}
                >
                    {link.label}
                </a>
            {/each}
        </nav>

        <button
            class="menu-toggle"
            aria-label="Toggle navigation menu"
            aria-expanded={menuOpen}
            onclick={() => (menuOpen = !menuOpen)}
        >
            <span class="bars" class:open={menuOpen}></span>
        </button>
    </header>

    {#if menuOpen}
        <nav class="nav-mobile" aria-label="Primary">
            {#each links as link}
                <a
                    class="navlink"
                    class:active={link.match(pathname)}
                    aria-current={link.match(pathname) ? "page" : undefined}
                    href={hrefOf(link)}
                    onclick={close}
                >
                    {link.label}
                </a>
            {/each}
        </nav>
    {/if}

    <main class="content">
        {@render children()}
    </main>
</div>

<style>
    .shell {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
    }

    .appbar {
        height: var(--app-bar-h);
        display: flex;
        align-items: center;
        gap: var(--sp-4);
        padding: 0 var(--sp-4);
        background: var(--c-surface);
        border-bottom: 1px solid var(--c-border);
        box-shadow: var(--shadow-sm);
        position: sticky;
        top: 0;
        z-index: 1000;
    }

    .brand {
        display: inline-flex;
        align-items: center;
        gap: var(--sp-2);
        font-weight: 600;
        font-size: var(--fs-lg);
        color: var(--c-text);
    }
    .brand:hover {
        text-decoration: none;
    }

    .nav-desktop {
        display: flex;
        align-items: center;
        gap: var(--sp-1);
        margin-left: auto;
    }

    .navlink {
        padding: 8px 14px;
        border-radius: var(--r-md);
        color: var(--c-text-muted);
        font-weight: 500;
    }
    .navlink:hover {
        background: var(--c-surface-2);
        color: var(--c-text);
        text-decoration: none;
    }
    .navlink.active {
        color: var(--c-accent-strong);
        background: var(--c-accent-soft);
    }

    .menu-toggle {
        display: none;
        margin-left: auto;
        width: 40px;
        height: 40px;
        border: 1px solid var(--c-border);
        border-radius: var(--r-md);
        background: var(--c-surface);
        cursor: pointer;
        position: relative;
    }
    .bars,
    .bars::before,
    .bars::after {
        content: "";
        position: absolute;
        left: 50%;
        width: 18px;
        height: 2px;
        background: var(--c-text);
        transform: translateX(-50%);
        transition: transform 0.2s ease, opacity 0.2s ease;
    }
    .bars {
        top: 50%;
        transform: translate(-50%, -50%);
    }
    .bars::before {
        top: -6px;
    }
    .bars::after {
        top: 6px;
    }
    .bars.open {
        background: transparent;
    }
    .bars.open::before {
        top: 0;
        transform: translateX(-50%) rotate(45deg);
    }
    .bars.open::after {
        top: 0;
        transform: translateX(-50%) rotate(-45deg);
    }

    .nav-mobile {
        display: flex;
        flex-direction: column;
        background: var(--c-surface);
        border-bottom: 1px solid var(--c-border);
        box-shadow: var(--shadow-md);
        position: sticky;
        top: var(--app-bar-h);
        z-index: 999;
    }
    .nav-mobile .navlink {
        border-radius: 0;
        padding: 14px var(--sp-4);
        border-bottom: 1px solid var(--c-border);
    }

    .content {
        flex: 1;
        min-height: 0;
    }

    @media (max-width: 768px) {
        .nav-desktop {
            display: none;
        }
        .menu-toggle {
            display: block;
        }
    }
</style>
