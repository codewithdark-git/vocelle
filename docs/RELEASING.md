# Releasing Vocelle

Vocelle uses [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`.

| Change | Bump | Example |
|---|---|---|
| Bug fixes only | patch | 0.2.0 → 0.2.1 |
| New features, backward compatible | minor | 0.2.1 → 0.3.0 |
| Breaking changes (settings format, removed features) | major | 0.9.0 → 1.0.0 |

While the version is below 1.0, minor releases may still change behavior.

Every release ships installers for Windows (`.exe`) and Linux (`.deb`, `.AppImage`), built by GitHub Actions from the version tag.

## Checklist

1. **Update the changelog.** Move the items under `[Unreleased]` in
   [CHANGELOG.md](../CHANGELOG.md) into a new section with the version and
   date, and update the comparison links at the bottom. This section becomes
   the release notes, so write it for users.

2. **Set the version** in `package.json`, `tauri.conf.json` and `Cargo.toml`:

   ```bash
   npm run version:set -- 0.3.0
   npm install
   cd src-tauri && cargo check && cd ..
   ```

3. **Run all checks** (the same ones CI runs):

   ```bash
   npm run check
   ```

4. **Preview the release notes:**

   ```bash
   node scripts/release-notes.mjs 0.3.0
   ```

5. **Commit and push, and wait for CI to pass on Windows and Linux:**

   ```bash
   git commit -am "Release v0.3.0"
   git push origin main
   ```

6. **Tag and push the tag:**

   ```bash
   git tag -a v0.3.0 -m "Vocelle 0.3.0"
   git push origin v0.3.0
   ```

   The [Release workflow](../.github/workflows/release.yml) checks that the tag
   matches the app version, creates a **draft** release with the generated
   notes, and attaches the Windows and Linux installers.

7. **Smoke-test the installers** from the draft: install, open Settings from
   the tray, download a model, dictate into a text editor, uninstall. Check the
   SHA-256 of what you downloaded against the draft's assets.

8. **Publish the draft:**

   ```bash
   gh release edit v0.3.0 --draft=false --latest
   ```

## Build notes

- whisper.cpp is compiled for an AVX2 baseline (see
  `src-tauri/.cargo/config.toml`), so installers behave the same no matter
  which machine builds them.
- Linux builds run on Ubuntu 22.04 so the AppImage works on older and newer
  distributions alike.
- Third-party GitHub Actions are pinned to commit SHAs; Dependabot proposes
  updates.
- The installers aren't code-signed yet. Once a certificate is available, set
  `bundle.windows.certificateThumbprint` in `tauri.conf.json`.
