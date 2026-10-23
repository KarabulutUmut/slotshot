import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  realpathSync,
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
  beginUpdate,
  removeBundle,
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
  const directory = realpathSync(
    mkdtempSync(join(tmpdir(), "slotshot-workflow-test-")),
  );
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

const bundleID = () => "io.github.karabulutumut.slotshot";

function updateFixture(t, previous = true) {
  const directory = fixture(t);
  const destination = join(directory, "Slotshot.app");
  if (previous) {
    mkdirSync(destination);
    writeFileSync(join(destination, "version"), "old");
  }
  const update = beginUpdate(directory);
  const staged = join(update, "Slotshot.app");
  mkdirSync(staged);
  writeFileSync(join(staged, "version"), "new");
  return { directory, destination, update, staged };
}

const noop = () => {};

test("successful verification precedes cleanup; repeated updates leave no archives", (t) => {
  const { directory, destination, update } = updateFixture(t);
  installBundle(
    update,
    destination,
    bundleID,
    () => {
      assert.equal(readFileSync(join(destination, "version"), "utf8"), "new");
      assert.equal(
        readFileSync(join(update, "previous/Slotshot.app/version"), "utf8"),
        "old",
      );
    },
    noop,
  );
  assert.deepEqual(readdirSync(directory), ["Slotshot.app"]);
  const next = beginUpdate(directory);
  mkdirSync(join(next, "Slotshot.app"));
  installBundle(next, destination, bundleID, noop, noop);
  assert.deepEqual(readdirSync(directory), ["Slotshot.app"]);
});

for (const failure of ["rename", "signature", "launch"]) {
  test(`${failure} failure restores previous app and retains staging for inspection`, (t) => {
    const { directory, destination, update, staged } = updateFixture(t);
    let stopped = false;
    assert.throws(
      () =>
        installBundle(
          update,
          destination,
          bundleID,
          () => {
            throw new Error(failure);
          },
          () => {
            stopped = true;
          },
          failure === "rename"
            ? () => {
                throw new Error(failure);
              }
            : undefined,
        ),
      new RegExp(failure),
    );
    assert.equal(readFileSync(join(destination, "version"), "utf8"), "old");
    assert.equal(readFileSync(join(staged, "version"), "utf8"), "new");
    assert.equal(stopped, failure !== "rename");
    assert.throws(() => beginUpdate(directory), /interrupted update/);
  });
}

test("failed quit keeps the working backup and the running replacement in place", (t) => {
  const { destination, update } = updateFixture(t);
  assert.throws(
    () =>
      installBundle(
        update,
        destination,
        bundleID,
        () => {
          throw new Error("launch");
        },
        () => {
          throw new Error("quit refused");
        },
      ),
    /quit refused/,
  );
  assert.equal(readFileSync(join(destination, "version"), "utf8"), "new");
  assert.equal(
    readFileSync(join(update, "previous/Slotshot.app/version"), "utf8"),
    "old",
  );
});

test("first install succeeds; failed first launch preserves staged bundle", (t) => {
  const ok = updateFixture(t, false);
  installBundle(ok.update, ok.destination, bundleID, noop, noop);
  assert.deepEqual(readdirSync(ok.directory), ["Slotshot.app"]);
  const failed = updateFixture(t, false);
  assert.throws(
    () =>
      installBundle(
        failed.update,
        failed.destination,
        bundleID,
        () => {
          throw new Error("launch");
        },
        noop,
      ),
    /launch/,
  );
  assert.ok(!existsSync(failed.destination));
  assert.ok(existsSync(failed.staged));
});

test("interrupted update, unknown contents, linked parents, and unrelated bundles are preserved", (t) => {
  const { directory, destination, update } = updateFixture(t);
  writeFileSync(join(update, "notes"), "valuable");
  assert.throws(() => beginUpdate(directory), /interrupted update/);
  assert.throws(
    () => installBundle(update, destination, bundleID, noop, noop),
    /Unexpected update/,
  );
  assert.equal(readFileSync(join(update, "notes"), "utf8"), "valuable");
  const linked = join(fixture(t), "linked");
  symlinkSync(directory, linked, "dir");
  assert.throws(() => beginUpdate(linked), /linked application/);
  assert.throws(
    () =>
      installBundle(update, join(directory, "Other.app"), bundleID, noop, noop),
    /unexpected installation paths/,
  );
  assert.throws(
    () => removeBundle(destination, () => "unrelated"),
    /unrelated application/,
  );
  assert.ok(existsSync(destination));
});

test("unrelated staged app and linked recovery directory are refused before replacement", (t) => {
  const { update, destination } = updateFixture(t);
  assert.throws(
    () => installBundle(update, destination, () => "unrelated", noop, noop),
    /unrelated application/,
  );
  const previous = join(update, "previous");
  rmSync(previous, { recursive: true });
  symlinkSync(fixture(t), previous, "dir");
  assert.throws(
    () => installBundle(update, destination, bundleID, noop, noop),
    /Unexpected update/,
  );
  assert.equal(readFileSync(join(destination, "version"), "utf8"), "old");
});

test("cleanup failure preserves verified install and recovery files", (t) => {
  const { update, destination } = updateFixture(t);
  let calls = 0;
  const read = () => (++calls > 2 ? "unrelated" : bundleID());
  assert.throws(
    () => installBundle(update, destination, read, noop, noop),
    /unrelated application/,
  );
  assert.equal(readFileSync(join(destination, "version"), "utf8"), "new");
  assert.equal(
    readFileSync(join(update, "previous/Slotshot.app/version"), "utf8"),
    "old",
  );
});
