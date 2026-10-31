# Project Rule: Rebuild the Frontend Before Committing Frontend Changes

The server embeds the frontend from the committed `static/` directory
(`RustEmbed`), and the production container is built from GitHub. This means
`static/` is a **build artifact that is checked into git**, and the running
app serves whatever `static/` contained at commit time — not the latest
`frontend/src`.

## The rule

Whenever you change anything under `frontend/` (any `.svelte`, `.js`, `.css`,
config, or dependency), you MUST regenerate and commit `static/` in the same
change. A commit that touches `frontend/src` but not `static/` ships stale
assets: the source looks fixed, but the container still serves the old build.

### How to rebuild

```
just build-ui
```

If `just` is unavailable, run the recipe steps directly:

```
cd frontend && PUBLIC_BASE_URL="" npm run build && cd ..
rm -rf static && mkdir static
cp -r frontend/.svelte-kit/output/prerendered/pages/* static/
cp -r frontend/.svelte-kit/output/client/* static/
git add static
touch src/http.rs   # force the embedded assets to recompile
```

### Before committing

- Confirm `static/` changed when `frontend/` did:
  `git status --short static/` should show modifications.
- Stage the rebuilt `static/` together with the source change, ideally in the
  same commit, so source and embedded bundle never diverge.
- After deploying, the server/container must be rebuilt and restarted for new
  `static/` assets to take effect; a running old binary keeps serving old
  assets.

## Why this exists

Live-position auto refresh was added to `MapView.svelte` and committed, but
`static/` was not rebuilt. The GitHub-built container therefore kept serving a
frontend with no polling code, and the feature appeared broken even though the
source and backend were correct. Always rebuild `static/` with frontend
changes.
