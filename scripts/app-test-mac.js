import { spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmdirSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const identifier = "io.github.karabulutumut.slotshot";

function run(command, args, capture = false, env = process.env) {
  const result = spawnSync(command, args, {
    cwd: root,
    env,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  if (result.error || result.status !== 0) {
    throw new Error(
      `${command} failed: ${result.error?.message ?? result.stderr?.trim() ?? `exit ${result.status}`}`,
    );
  }
  return result.stdout?.trim();
}

export function signingIdentity(output, requested) {
  const identities = [...output.matchAll(/\b([A-Fa-f0-9]{40}) "([^"]+)"/g)].map(
    ([, hash, name]) => ({ hash, name }),
  );
  if (requested === "-") return "-";
  if (requested) {
    const matches = identities.filter(
      ({ hash, name }) =>
        hash.toLowerCase() === requested.toLowerCase() || name === requested,
    );
    if (matches.length !== 1) {
      throw new Error(
        "SLOTSHOT_SIGNING_IDENTITY must name exactly one valid codesigning identity (with its private key) from security find-identity -v -p codesigning, or '-' for ad-hoc signing.",
      );
    }
    return matches[0].hash;
  }
  const development = identities.filter(({ name }) =>
    name.startsWith("Apple Development:"),
  );
  if (development.length > 1) {
    throw new Error(
      "Multiple Apple Development identities found. Set SLOTSHOT_SIGNING_IDENTITY to the certificate SHA-1 shown by security find-identity -v -p codesigning.",
    );
  }
  return development[0]?.hash ?? "-";
}

export function assertBundle(path, readIdentifier) {
  const stat = lstatSync(path, { throwIfNoEntry: false });
  if (!stat) return;
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`Refusing to replace a symlink or non-directory: ${path}`);
  }
  if (readIdentifier(path) !== identifier) {
    throw new Error(`Refusing to replace an unrelated application: ${path}`);
  }
}

// Keep previous bundles for manual recovery; never delete an existing app.
export function archiveBundle(path) {
  if (!existsSync(path)) return null;
  const backup = join(
    mkdtempSync(join(dirname(path), ".slotshot-previous-")),
    "Slotshot.app",
  );
  renameSync(path, backup);
  console.log(`Previous bundle retained at ${backup}`);
  return backup;
}

export function installBundle(staged, destination, move = renameSync) {
  const backup = archiveBundle(destination);
  try {
    move(staged, destination);
  } catch (error) {
    if (backup) renameSync(backup, destination);
    throw error;
  }
}

function main() {
  if (process.platform !== "darwin") {
    throw new Error(
      "app:test:mac requires macOS. Use the normal Tauri workflow on Windows.",
    );
  }
  const applications = join(homedir(), "Applications");
  const destination = join(applications, "Slotshot.app");
  const buildApp = join(
    root,
    "src-tauri/target/debug/bundle/macos/Slotshot.app",
  );
  const readIdentifier = (app) =>
    run(
      "/usr/libexec/PlistBuddy",
      ["-c", "Print :CFBundleIdentifier", join(app, "Contents/Info.plist")],
      true,
    );
  const config = JSON.parse(
    readFileSync(join(root, "src-tauri/tauri.conf.json")),
  );
  if (config.identifier !== identifier || config.productName !== "Slotshot") {
    throw new Error(
      "The canonical workflow requires the existing Slotshot name and identifier to preserve storage and permissions.",
    );
  }
  run("xcode-select", ["-p"], true);
  run("xcrun", ["--find", "swift"], true);
  run("cargo", ["--version"], true);
  if (!existsSync(join(root, "node_modules/.bin/tauri"))) {
    throw new Error("Missing npm dependencies. Run npm ci first.");
  }
  const identity = signingIdentity(
    run(
      "/usr/bin/security",
      ["find-identity", "-v", "-p", "codesigning"],
      true,
    ),
    process.env.SLOTSHOT_SIGNING_IDENTITY,
  );
  console.log(
    `Testing app: ${destination}\nBundle identifier: ${identifier}\nSigning identity: ${identity}`,
  );
  if (identity === "-") {
    console.warn(
      "AD-HOC FALLBACK: no stable developer identity selected. Screen Recording permission may need renewal after each changed build. See README for Xcode certificate setup.",
    );
  }
  mkdirSync(applications, { recursive: true });
  assertBundle(destination, readIdentifier);
  assertBundle(buildApp, readIdentifier);
  mkdirSync(join(root, "src-tauri/target"), { recursive: true });
  const lock = join(root, "src-tauri/target/.app-test-mac-lock");
  try {
    mkdirSync(lock);
  } catch {
    throw new Error(
      `Another app:test:mac run may be active. If a previous run was interrupted, confirm it has stopped, then remove the empty lock directory ${lock}.`,
    );
  }
  try {
    const native = (mode) =>
      run("xcrun", [
        "swift",
        join(root, "scripts/app-test-mac.swift"),
        mode,
        destination,
        identifier,
      ]);
    // Refuse other builds before touching their bundle or shared storage lock.
    native("stop");
    archiveBundle(buildApp);
    // Sign explicitly after bundling: no certificate import, notarization, DMG,
    // updater artifact, or inherited release-signing environment is needed.
    const env = { ...process.env };
    for (const key of Object.keys(env)) {
      if (
        key.startsWith("APPLE_") ||
        key.startsWith("TAURI_SIGNING_") ||
        key === "CARGO_TARGET_DIR"
      )
        delete env[key];
    }
    run(
      "npm",
      [
        "run",
        "tauri",
        "--",
        "build",
        "--debug",
        "--bundles",
        "app",
        "--config",
        JSON.stringify({
          bundle: {
            macOS: { signingIdentity: "-" },
            createUpdaterArtifacts: false,
          },
        }),
        "--",
        "--locked",
      ],
      false,
      env,
    );
    assertBundle(buildApp, readIdentifier);
    if (!existsSync(buildApp))
      throw new Error(`Build did not produce ${buildApp}`);
    const staging = mkdtempSync(join(applications, ".slotshot-staging-"));
    const staged = join(staging, "Slotshot.app");
    run("/usr/bin/ditto", [buildApp, staged]);
    run("/usr/bin/codesign", [
      "--force",
      "--sign",
      identity,
      "--identifier",
      identifier,
      "--timestamp=none",
      staged,
    ]);
    run("/usr/bin/codesign", [
      "--verify",
      "--deep",
      "--strict",
      "--verbose=2",
      staged,
    ]);
    // A second check catches another build opened while compiling. No force kill.
    native("stop");
    assertBundle(destination, readIdentifier);
    installBundle(staged, destination);
    rmdirSync(staging);
    run("/usr/bin/codesign", ["-dvv", "-r-", destination]);
    run("/usr/bin/open", ["-a", destination]);
    native("verify");
    console.log(
      "Canonical Slotshot launched. Authorize this exact app in Screen & System Audio Recording, then fully quit/reopen if macOS requests it. No TCC settings or screenshot data were reset.",
    );
  } finally {
    rmdirSync(lock);
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main();
  } catch (error) {
    console.error(`app:test:mac: ${error.message}`);
    process.exitCode = 1;
  }
}
