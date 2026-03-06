---
name: release
description: Cut a new whisper-type release - run tests, bump version in Cargo.toml, PR develop→main, tag vX.Y.Z
disable-model-invocation: true
---

Steps to cut a release:

1. Verify the working tree is clean (`git status`) and on the `develop` branch. Stop and report if not.

2. Run the full test suite and linter:
   ```
   cargo test
   cargo clippy -- -D warnings
   ```
   Stop if either fails — do not proceed with a broken build.

3. Ask the user for the new version number (e.g. `0.2.0`).

4. Update `version = "..."` in `Cargo.toml` to the new version. Run `cargo build` to regenerate `Cargo.lock`.

5. Stage and commit:
   ```
   git add Cargo.toml Cargo.lock
   git commit -m "chore: bump version to vX.Y.Z"
   ```

6. Push the `develop` branch and open a PR from `develop` → `main` with:
   - Title: `Release vX.Y.Z`
   - Body: summary of changes since the last tag (`git log <last-tag>..HEAD --oneline`)

7. Wait for the user to confirm the PR is merged. Then:
   ```
   git checkout main && git pull
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   git checkout develop
   ```

8. Report the tag URL and confirm completion.
