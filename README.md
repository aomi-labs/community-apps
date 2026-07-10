# Aomi [*]-apps Template

This repository is a GitHub template for Aomi platform app release builders.
Generate a new repository from it when you need an app artifact builder such as
`boreal-apps`, `partner-apps`, or another `[*]-apps` platform repo.

The template intentionally ships with no staged apps. The `apps/` directory is
kept only by `apps/.gitkeep`; backend deploy flows populate real app directories
later.

## After Creating A Repository From This Template

1. Rename every `[*]-apps` placeholder in the generated repo docs and
   `platform.json` to the new GitHub repo name.
2. Set `platform.json.name` to the platform slug that app manifests will use in
   `aomi.toml`.
3. Confirm `platform.json.source_repo` matches `<owner>/<repo>`.
4. Confirm `platform.json.required_sdk_version` and `default_target` match the
   backend/runtime you are deploying to.
5. Mark the generated GitHub repository as a template too if it should itself be
   reusable.

## Repository Layout

```
[*]-apps/
|-- README.md
|-- CONTRIBUTING.md
|-- platform.json
|-- apps/
|   `-- .gitkeep
`-- .github/
    |-- workflows/build-candidate.yml
    `-- scripts/build_candidate.py
```

When the backend stages an app, it writes directories shaped like:

```
apps/<installation-id>/<repo-key>/<app>/
|-- .aomi/deployment.json
|-- aomi.toml
|-- Cargo.toml
`-- src/
```

## Release Builder Contract

This repository is only the artifact builder. The backend owns source access,
deployment records, candidate branch creation, release tag selection, and
activation.

CI expects backend-generated candidate branches shaped like:

```
<source-owner>/<source-repo>/<installation-id>/<short-source-commit>
```

For each changed staged app under `apps/<installation-id>/<repo-key>/<app>/`,
CI validates `.aomi/deployment.json`, builds the app as a Rust `cdylib`, and
publishes release assets:

- `aomi-plugins-<release-tag>-<target>.tar.gz`
- `manifest.json`
- `aomi-release.json`

## Platform Descriptor

`platform.json` is the static release-builder configuration for the generated
platform repository.

| Field | Meaning |
|---|---|
| `name` | Platform slug. App manifests must use this value as `platform`. |
| `source_repo` | GitHub repository name for this builder, for example `aomi-labs/[*]-apps`. |
| `publish_branch` | Protected baseline branch used by backend candidate PRs and CI diffs. |
| `app_path_prefix` | Directory where staged apps are written. |
| `app_path_convention` | Expected staged app path shape. |
| `release_tag_convention` | Expected release tag shape. |
| `visibility` | Intended release visibility for this platform. |
| `review_policy` | Human-readable review policy label. |
| `required_sdk_version` | Exact `aomi-sdk` version app crates must pin. |
| `default_target` | Rust target triple CI builds. |

## Related

- [`aomi-sdk`](https://github.com/aomi-labs/aomi-sdk) - SDK and `aomi-build`
- [`aomi-launch-my-agent`](https://github.com/aomi-labs/aomi-launch-my-agent) -
  deploy and activation contract notes
