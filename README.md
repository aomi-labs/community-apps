# Aomi Community Apps

This repo hosts the **source for community Aomi apps**. If you're a contributor
who wants to ship an app on the Aomi runtime, you're in the right place.

You **do not hand-edit `apps/<your-slug>/`**. That directory is generated for
you by the `aomi-git` CLI from your own source repo. This repo is the publishing
target — it's where releases get cut from.

## Contributing an app

👉 **Read [`CONTRIBUTING.md`](./CONTRIBUTING.md) first.** It walks you through
the full pipeline from "I have a Rust crate" to "my app is loaded on staging"
in under 10 minutes.

The short version:

1. Author your app in your own source repo with an `aomi.toml` declaring
   `platform = "community"` and `git = "https://github.com/aomi-labs/community-apps"`.
2. Run `aomi-git deploy --platform-repo-dir /path/to/this/repo`. It stages your
   source under `apps/<slug>/`, commits, and pushes to the `publish` branch.
3. GitHub Actions builds the cdylib and uploads a release tarball tagged
   `apps-<slug>-<short-source-commit>`.
4. Ping the platform operator with your release tag; they activate it against
   the backend.

If you opened a PR by hand-editing `apps/<slug>/` directly, **please redo via
`aomi-git deploy`** — the publish CI validates the file shapes that `aomi-git`
emits, and hand-edited PRs will fail at validate time.

## Repo layout

```
community-apps/
├── README.md           ← you are here
├── CONTRIBUTING.md     ← E2E contributor guide
├── apps/               ← generated source per app; one dir per slug
│   ├── alice-bot/
│   ├── fanforge/
│   └── gambit/
├── ci/
│   └── platform.json   ← CI contract: required SDK version, build target, etc.
├── fixtures/
│   └── hello-ci/       ← buildable crate used by maintainers for ad-hoc dry-runs
└── scripts/
    └── publish_app.py  ← internal build script driven by Actions; not for contributors
```

## Publication contract

These facts are enforced by CI; they exist for reference. Contributors don't
need to memorize them.

| Field | Value |
|---|---|
| Publication branch | `publish` |
| Staged app path | `apps/<app_slug>/` |
| Build contract file | `apps/<app_slug>/.aomi/deployment.json` (written by `aomi-git deploy`) |
| Release tag convention | `apps-{app_slug}-{short_commit}` |
| Runtime bundle contract | `aomi-plugin-bundle-v1` |
| Required SDK version | see [`platform.json`](./platform.json) |

`short_commit` is the first 12 characters of your source commit recorded by
`aomi-git`.

Each release contains:

- `aomi-plugins-{app_slug}-{short_commit}-{target}.tar.gz` — the runtime bundle
- `manifest.json` — release metadata (mirrors `plugins/manifest.json` inside the tarball)
- `aomi-release.json` — provenance metadata (not a runtime trust boundary)

The backend trusts a release only after `PluginFetcher` validates the release
tag, exact SDK version, build target, and plugin SHA-256 hashes inside the
tarball.

## Build internals

CI is driven by [`scripts/publish_app.py`](./scripts/publish_app.py), invoked
from [`.github/workflows/publish-apps.yml`](./.github/workflows/publish-apps.yml)
on push to `publish`. Contributors don't run either directly — `aomi-git
deploy` and the workflow handle it.

## Related

- [`aomi-sdk`](https://github.com/aomi-labs/aomi-sdk) — the SDK and the
  `aomi-git` deploy CLI
- [`aomi-launch-my-agent`](https://github.com/aomi-labs/aomi-launch-my-agent) —
  ADRs for the deploy/activate contract (especially 0004, 0009, 0010)
