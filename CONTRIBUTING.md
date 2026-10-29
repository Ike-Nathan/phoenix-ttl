# Contributing to Phoenix

Thanks for your interest! Phoenix is a small crate and every contribution is reviewed by a human. This guide covers setup, how to pick up work, and what a mergeable pull request looks like.

## Setup

You need **Rust 1.91 or later** (the minimum for soroban-sdk 29) and git.

```sh
git clone https://github.com/Ike-Nathan/phoenix-ttl && cd phoenix-ttl
cargo test --workspace
```

| Command | What it does |
| --- | --- |
| `cargo test --workspace` | Unit tests, integration tests (`tests/`), doc tests and the example contract's tests. |
| `cargo test -p phoenix-example-vault -- --nocapture` | Run the example and see the reports. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lints. CI uses `-D warnings`. |
| `cargo fmt --all` | Format. |
| `cargo doc -p phoenix-ttl --open` | Read the API docs you are changing. |

## Picking up an issue

1. Find an issue labelled `good first issue` or `help wanted`.
2. **Comment to ask to be assigned, and wait until you are assigned before starting.** Unassigned PRs for issues someone else is assigned to will be closed.
3. If you can't continue, say so in the issue so it can be reassigned. Issues with no activity for 7 days may be unassigned.
4. For new ideas, open a feature request first and wait for the scope to be confirmed.

## Pull requests

- One issue per PR, kept focused.
- [Conventional Commits](https://www.conventionalcommits.org/) for commit messages and PR titles: `feat:`, `fix:`, `docs:`, `test:`, `chore:`, `refactor:`, `ci:`.
- Tests are required. Checks need a passing and a failing case against a real contract in `tests/phoenix.rs`. Bug fixes need a test that fails without the fix.
- Report text is user-facing. If you change it, update the tests that check it and the README example output.
- Don't add dependencies without discussing it in the issue first. Phoenix currently depends only on `soroban-sdk`.
- Update `README.md`, `docs/` and `CHANGELOG.md` (Unreleased) when behaviour changes.
- Never state SDK or protocol behaviour you haven't checked. Link the docs or SDK source you relied on in the PR description.

See [docs/architecture.md](docs/architecture.md) and [docs/extending.md](docs/extending.md).

## Drips Wave contributors

Phoenix takes part in the [Drips Stellar Wave](https://www.drips.network/wave/stellar). Eligible issues carry a complexity label that sets the points awarded when the PR is merged:

| Label | Points | Typical scope |
| --- | --- | --- |
| `complexity: trivial` | 100 | Docs fixes, a single test, small API polish. |
| `complexity: medium` | 150 | A self-contained feature or bug fix with tests. |
| `complexity: high` | 200 | A larger feature across modules (new check family, report format behind a feature flag), with tests and docs. |

- Only the **assigned** contributor's PR is eligible.
- Points are awarded for **merged** PRs that resolve the issue as described.
- The maintainer may adjust a complexity label before assignment. The label at assignment time counts.
- Low-effort or automated submissions that don't build or don't follow the issue will be closed.

## Security

Do not open public issues for security problems. See [SECURITY.md](SECURITY.md).

## Code of conduct

Everyone taking part is expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
