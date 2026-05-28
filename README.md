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
├── platform.json       ← platform descriptor (see below)
├── apps/               ← generated source per app; one dir per slug
│   ├── alice-bot/
│   ├── fanforge/
│   └── gambit/
└── fixtures/
    └── hello-world/    ← buildable crate template; never deployed
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

## Platform descriptor (`platform.json`)

`platform.json` at the repo root is the **platform contract** — every rule
your app must meet to publish here. It's hand-authored by the platform
operator and read by CI on every push.

| Field | Meaning | Touched by |
|---|---|---|
| `name` | Platform tier label (`community`). Match in your `aomi.toml` as `platform = "community"`. | Operator on platform bring-up |
| `source_repo` | This repo (`aomi-labs/community-apps`). CI verifies your `aomi.toml`'s `git` resolves here. | Operator |
| `publish_branch` | The branch `aomi-git deploy` pushes to. Protected against force-push and deletion. | Operator |
| `app_path_prefix` | Where staged apps land (`apps`). Combined with your slug → `apps/<slug>/`. | Operator |
| `release_tag_convention` | Pattern for GitHub release tags built from your source commit. | Operator |
| `visibility` | `public` for this repo — release tarballs are world-readable. | Operator |
| `review_policy` | `community-review` — describes how PRs / contributions are vetted. Informational. | Operator |
| `required_sdk_version` | **The aomi-sdk version your app MUST pin in `Cargo.toml`.** Bundle validation fails on mismatch. | Operator on SDK bumps |
| `default_target` | Rust target triple CI builds for (`x86_64-unknown-linux-gnu`). | Operator |

You (the contributor) don't edit `platform.json`. You **read** the
`required_sdk_version` and pin it in your `Cargo.toml`. That's it.

When the platform operator bumps `required_sdk_version`, you'll need to
update your app's pin to match before your next deploy.

## Build internals

The publish workflow at
[`.github/workflows/publish-apps.yml`](./.github/workflows/publish-apps.yml)
runs on push to `publish` and drives a small Python script tucked under
`.github/scripts/` that no contributor (or anyone) runs by hand — `aomi-git
deploy` and the workflow handle everything.

## Related

- [`aomi-sdk`](https://github.com/aomi-labs/aomi-sdk) — the SDK and the
  `aomi-git` deploy CLI
- [`aomi-launch-my-agent`](https://github.com/aomi-labs/aomi-launch-my-agent) —
  ADRs for the deploy/activate contract (especially 0004, 0009, 0010)
