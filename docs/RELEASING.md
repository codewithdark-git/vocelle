# Releasing Vocelle

Vocelle uses [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`.

| Change | Bump | Example |
|---|---|---|
| Bug fixes only | patch | 0.1.0 → 0.1.1 |
| New features, backward compatible | minor | 0.1.1 → 0.2.0 |
| Breaking changes (settings format, removed features) | major | 0.9.0 → 1.0.0 |

While the version is below 1.0, minor releases may still change behavior.

## Checklist

1. **Update the changelog.** Move the items under `[Unreleased]` in
   [CHANGELOG.md](../CHANGELOG.md) into a new section with the version and
   date, and update the comparison links at the bottom.

2. **Set the version** in `package.json`, `tauri.conf.json` and `Cargo.toml`:

   ```bash
   npm run version:set -- 0.2.0
   npm install
   ```

3. **Run all checks** (the same ones CI runs):

   ```bash
   npm run check
   ```

4. **Build the installer:**

   ```bash
   npm run tauri build
   ```

   The installer is written to
   `src-tauri/target/release/bundle/nsis/Vocelle_<version>_x64-setup.exe`.

5. **Smoke-test the installer** on a clean account or VM: install, open
   Settings from the tray, download a model, dictate into Notepad, uninstall.

6. **Commit, tag and push:**

   ```bash
   git commit -am "Release v0.2.0"
   git tag -a v0.2.0 -m "Vocelle 0.2.0"
   git push origin main --follow-tags
   ```

7. **Publish the GitHub release** with the installer attached. Use the
   version's changelog section as the release notes:

   ```bash
   gh release create v0.2.0 \
     "src-tauri/target/release/bundle/nsis/Vocelle_0.2.0_x64-setup.exe" \
     --title "Vocelle 0.2.0" --notes-file release-notes.md
   ```

## Build notes

- whisper.cpp is compiled for an AVX2 baseline (see
  `src-tauri/.cargo/config.toml`), so installers behave the same no matter
  which machine builds them.
- The installer isn't code-signed yet. Once a certificate is available, set
  `bundle.windows.certificateThumbprint` in `tauri.conf.json`.
