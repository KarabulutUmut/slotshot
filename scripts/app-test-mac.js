import { spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
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

// A fixed directory bounds recovery artifacts to one update. Never sweep older
// random archives or a pre-existing transaction: interruption requires inspection.
export function beginUpdate(applications) {
  const stat = lstatSync(applications);
  if (
    !stat.isDirectory() ||
    stat.isSymbolicLink() ||
    realpathSync(applications) !== applications
  ) {
    throw new Error(`Refusing a linked application directory: ${applications}`);
  }
  const update = join(applications, ".slotshot-update");
  try {
    mkdirSync(update, { mode: 0o700 });
  } catch (error) {
    throw new Error(
      `Cannot start update at ${update}: ${error.message}. An interrupted update may contain the last working app in previous/Slotshot.app. Confirm no build is active and inspect it before recovering/removing it; nothing was deleted.`,
      { cause: error },
    );
  }
  mkdirSync(join(update, "previous"));
  return update;
}

export function removeBundle(path, readIdentifier) {
  if (realpathSync(dirname(path)) !== dirname(path))
    throw new Error(`Refusing a linked bundle parent: ${path}`);
  assertBundle(path, readIdentifier);
  if (existsSync(path)) rmSync(path, { recursive: true });
}

export function installBundle(
  update,
  destination,
  readIdentifier,
  verify,
  stop,
  move = renameSync,
) {
  // Both paths must be exact children of the same non-linked Applications dir.
  const applications = dirname(update);
  if (
    realpathSync(applications) !== applications ||
    update !== join(realpathSync(applications), ".slotshot-update") ||
    destination !== join(realpathSync(applications), "Slotshot.app")
  ) {
    throw new Error("Refusing unexpected installation paths.");
  }
  if (lstatSync(update).isSymbolicLink())
    throw new Error("Refusing a linked update directory.");
  const staged = join(update, "Slotshot.app");
  const previousDirectory = join(update, "previous");
  const backup = join(previousDirectory, "Slotshot.app");
  if (
    lstatSync(previousDirectory).isSymbolicLink() ||
    readdirSync(previousDirectory).length !== 0 ||
    readdirSync(update).some(
      (name) => !["Slotshot.app", "previous"].includes(name),
    )
  ) {
    throw new Error("Unexpected update contents; inspect before retrying.");
  }
  assertBundle(staged, readIdentifier);
  if (!existsSync(staged)) throw new Error("Missing staged application.");
  assertBundle(destination, readIdentifier);
  const hadPrevious = existsSync(destination);
  if (hadPrevious) renameSync(destination, backup);
  try {
    move(staged, destination);
    verify(destination);
  } catch (error) {
    // If quitting fails, keep both bundles and the transaction for manual recovery.
    if (existsSync(destination)) {
      stop();
      assertBundle(destination, readIdentifier);
      renameSync(destination, staged);
    }
    if (hadPrevious) renameSync(backup, destination);
    throw error;
  }
  // Cleanup is deliberately outside rollback: a cleanup error must never replace
  // a successfully verified app. Keep the fixed directory for inspection/retry.
  removeBundle(backup, readIdentifier);
  rmdirSync(previousDirectory);
  rmdirSync(update);
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
  const readIdentifier = (app) => {
    for (const [relative, directory] of [
      ["Contents", true],
      ["Contents/MacOS", true],
      ["Contents/Info.plist", false],
      ["Contents/MacOS/slotshot", false],
    ]) {
      const stat = lstatSync(join(app, relative));
      if (
        stat.isSymbolicLink() ||
        (directory ? !stat.isDirectory() : !stat.isFile())
      )
        throw new Error(`Unexpected bundle structure: ${app}/${relative}`);
    }
    const executable = run(
      "/usr/libexec/PlistBuddy",
      ["-c", "Print :CFBundleExecutable", join(app, "Contents/Info.plist")],
      true,
    );
    if (executable !== "slotshot")
      throw new Error(`Unexpected executable: ${app}`);
    return run(
      "/usr/libexec/PlistBuddy",
      ["-c", "Print :CFBundleIdentifier", join(app, "Contents/Info.plist")],
      true,
    );
  };
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
    const update = beginUpdate(applications);
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
    removeBundle(buildApp, readIdentifier);
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
    const staged = join(update, "Slotshot.app");
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
    installBundle(
      update,
      destination,
      readIdentifier,
      (app) => {
        run("/usr/bin/codesign", ["--verify", "--deep", "--strict", app]);
        run("/usr/bin/codesign", ["-dvv", "-r-", app]);
        run("/usr/bin/open", ["-a", app]);
        native("verify");
      },
      () => native("stop"),
    );
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
