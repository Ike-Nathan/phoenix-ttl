# Releasing

Releases are cut by the maintainer. Nothing is published automatically.

1. Make sure `main` is green.
2. Update `version` in `Cargo.toml` and move the Unreleased section of `CHANGELOG.md` under the new version.
3. Run the checks in [extending.md](extending.md#checks-before-opening-a-pr) and `cargo package -p phoenix-ttl`.
4. Commit (`chore: release vX.Y.Z`), tag and push:
   ```sh
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin main vX.Y.Z
   ```
5. Create the GitHub release from the tag with the changelog section.
6. After review: `cargo publish -p phoenix-ttl`. Check `cargo package --list -p phoenix-ttl` first. `examples/`, `docs/` and `.github/` are excluded from the crate.
