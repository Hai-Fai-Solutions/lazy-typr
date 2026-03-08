---
name: release
description: Cut a new whisper-type release - run tests, open PR from feature/ or bugfix/ branch to main; CI handles version bump, tagging, and publishing
disable-model-invocation: true
---

Steps to cut a release:

1. Verify the working tree is clean (`git status`) and on a `feature/` or `bugfix/` branch. Stop and report if not.

2. Run the full test suite and linter:
   ```
   cargo test
   cargo clippy -- -D warnings
   ```
   Stop if either fails — do not proceed with a broken build.

3. Push the current branch and open a PR to `main` with:
   - Title: `Release` (CI determines the exact version from conventional commits)
   - Body: summary of unreleased changes (`git log $(git describe --tags --abbrev=0)..HEAD --oneline`)

4. Wait for the user to confirm the PR is merged.

   CI will automatically:
   - Run `cog bump --auto` to determine the new semver version from commit types
   - Update `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`
   - Commit with `chore(version): X.Y.Z [skip ci]` and push an annotated tag
   - Trigger `release.yml` to build, strip, and publish the GitHub Release

5. Report that the release pipeline is running and share the Actions URL for the user to monitor.

6. Remind the user to delete the source branch after the PR is merged (GitHub can do this automatically, or manually: `git push origin --delete <branch-name>` and `git branch -d <branch-name>`).
