# Contributing to Pelita

Pelita welcomes help from church volunteers, worship leaders, testers, translators, writers, designers, and developers. Small, specific reports from a real rehearsal are valuable.

## Report a bug

[Open a GitHub issue](https://github.com/DyDxdYdX/Pelita/issues). Include the Pelita version, operating system, display setup, what you expected, what happened, and steps to repeat it. For media problems, include the file type and codec if known. Share a small rights-cleared sample if needed; do not upload copyrighted worship content without permission.

## Request a feature

[Open an issue](https://github.com/DyDxdYdX/Pelita/issues) describing the worship task, who needs it, and how you handle it today. Check existing issues and the [product plans](docs/developer/README.md) first.

## Help without writing code

Try a full service rehearsal and report confusing steps. Improve the [user guide](docs/user/README.md), check instructions on another operating system, suggest clearer wording or translations, or contribute verified screenshots using invented or cleared content. Please say which app version a screenshot shows.

## Develop locally

The desktop app lives in `app/`. Install the platform prerequisites listed in [development setup](docs/development/development-setup.md), then use the locked npm dependencies and Tauri CLI:

```sh
cd app
npm ci
npx --yes @tauri-apps/cli@2.12.4 dev
```

See [developer documentation](docs/developer/README.md) for architecture, testing, release notes, and historical decisions. For a significant module or architecture change, follow [AGENTS.md](AGENTS.md): update the relevant design documentation before implementation and record deviations afterward.

## Pull requests

Keep each pull request focused. Explain the user problem, what changed, and how you checked it. Update user documentation when visible behavior changes. Run the relevant checks described in the [testing strategy](docs/development/testing-strategy.md); if a hardware test is unavailable, state that clearly. Respect the [MIT license](LICENSE) for project code and the separate [content permissions](docs/legal/content-permissions.md) for songs, recordings, images, and other media.
