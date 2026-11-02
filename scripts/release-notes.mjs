// Prints GitHub release notes for a version: install steps plus that
// version's CHANGELOG section. Usage: node scripts/release-notes.mjs 0.2.0
import { readFileSync } from "node:fs";

const version = (process.argv[2] ?? "").replace(/^v/, "");
if (!version) {
  console.error("Usage: node scripts/release-notes.mjs <version>");
  process.exit(1);
}

const changelog = readFileSync("CHANGELOG.md", "utf8");
const start = changelog.indexOf(`## [${version}]`);
if (start === -1) {
  console.error(`CHANGELOG.md has no section for ${version}`);
  process.exit(1);
}
const next = changelog.indexOf("\n## [", start + 1);
const section = changelog
  .slice(start, next === -1 ? undefined : next)
  .split("\n")
  .slice(1) // drop the "## [x.y.z] - date" heading; the release title has it
  .join("\n")
  .replace(/\n\[[^\]]+\]: https?:\/\/\S+/g, "") // link references belong to the changelog
  .trim();

const repo = "https://github.com/codewithdark-git/vocelle";
console.log(`## Download

| Platform | File |
|---|---|
| **Windows 10/11** (64-bit) | \`Vocelle_${version}_x64-setup.exe\` |
| **Linux** (Debian, Ubuntu) | \`Vocelle_${version}_amd64.deb\` |
| **Linux** (any distro) | \`Vocelle_${version}_amd64.AppImage\` |

**Windows:** run the installer. It isn't code-signed yet, so if Windows shows *"Windows protected your PC"*, click **More info → Run anyway**.

**Linux:** \`sudo apt install ./Vocelle_${version}_amd64.deb\`, or make the AppImage executable (\`chmod +x\`) and run it. Linux support is a preview: the talk key needs an X11 session.

Then open **Settings** from the tray icon, download the recommended speech model, and hold your talk key (Right Ctrl by default) to dictate. Full guide: [README](${repo}#quick-start).

${section}

**Full changelog:** [CHANGELOG.md](${repo}/blob/main/CHANGELOG.md)`);
