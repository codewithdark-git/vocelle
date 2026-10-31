// Sets the app version everywhere it is declared, so releases never drift.
// Usage: npm run version:set -- 0.2.0
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? "")) {
  console.error("Usage: npm run version:set -- <major.minor.patch>");
  process.exit(1);
}

function update(path, pattern, replacement) {
  const before = readFileSync(path, "utf8");
  const after = before.replace(pattern, replacement);
  if (after === before && !before.match(pattern)) {
    console.error(`No version field found in ${path}`);
    process.exit(1);
  }
  writeFileSync(path, after);
  console.log(`${path}: ${version}`);
}

update("package.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
update("src-tauri/tauri.conf.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
// Only the [package] version, which is the first `version =` line.
update("src-tauri/Cargo.toml", /^(version\s*=\s*")[^"]+(")/m, `$1${version}$2`);

console.log("Now run `npm install` and `cargo check` to refresh the lock files.");
