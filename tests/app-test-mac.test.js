import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  assertBundle,
  installBundle,
  signingIdentity,
} from "../scripts/app-test-mac.js";

const hash = "A".repeat(40);
const other = "B".repeat(40);
const identities = `1) ${hash} "Apple Development: Test (TEAM)"\n2) ${other} "Developer ID Application: Test (TEAM)"`;

test("prefer Apple Development, validate explicit identity, and label ad-hoc fallback", () => {
  assert.equal(signingIdentity(identities), hash);
  assert.equal(signingIdentity(identities, other.toLowerCase()), other);
  assert.equal(
    signingIdentity(identities, "Apple Development: Test (TEAM)"),
    hash,
  );
  assert.equal(signingIdentity("0 valid identities found"), "-");
  assert.equal(signingIdentity(identities, "-"), "-");
  assert.throws(
    () => signingIdentity(identities, "missing"),
    /valid codesigning identity/,
  );
  assert.throws(
    () =>
      signingIdentity(
        `${identities}\n3) ${"C".repeat(40)} "Apple Development: Other (OTHER)"`,
      ),
    /Multiple Apple Development/,
  );
});

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), "slotshot-workflow-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  return directory;
}

test("refuse symlink destinations and unrelated applications", (t) => {
  const directory = fixture(t);
  const app = join(directory, "Other.app");
  mkdirSync(app);
  assert.throws(
    () => assertBundle(app, () => "unrelated"),
    /unrelated application/,
  );
  assertBundle(app, () => "io.github.karabulutumut.slotshot");
  const link = join(directory, "Slotshot.app");
  symlinkSync(app, link, "dir");
  assert.throws(
    () => assertBundle(link, () => "io.github.karabulutumut.slotshot"),
    /symlink/,
  );
  const dangling = join(directory, "Dangling.app");
  symlinkSync(join(directory, "missing"), dangling, "dir");
  assert.throws(() => assertBundle(dangling, () => "unused"), /symlink/);
});

test("replacement retains the previous app; failed replacement restores it", (t) => {
  const directory = fixture(t);
  const destination = join(directory, "Slotshot.app");
  const staged = join(directory, "staged.app");
  mkdirSync(destination);
  mkdirSync(staged);
  writeFileSync(join(destination, "version"), "old");
  writeFileSync(join(staged, "version"), "new");
  assert.throws(
    () =>
      installBundle(staged, destination, () => {
        throw new Error("disk failure");
      }),
    /disk failure/,
  );
  assert.equal(readFileSync(join(destination, "version"), "utf8"), "old");
  assert.ok(existsSync(staged));
  installBundle(staged, destination);
  assert.equal(readFileSync(join(destination, "version"), "utf8"), "new");
  assert.ok(!existsSync(staged));
  const retained = readdirSync(directory)
    .filter((name) => name.startsWith(".slotshot-previous-"))
    .map((name) => join(directory, name, "Slotshot.app", "version"))
    .filter(existsSync);
  assert.equal(retained.length, 1);
  assert.equal(readFileSync(retained[0], "utf8"), "old");
});
