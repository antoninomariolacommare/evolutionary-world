import assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import { tmpdir } from "node:os";
import { gunzipSync } from "node:zlib";
import * as vm from "node:vm";
import test from "node:test";
import * as safety from "./world-safety";

// Emitted and accepted by the schema-8 Rust engine. See the fixture README.
const checkpoint = gunzipSync(fs.readFileSync(path.join(
  __dirname, "../../main/test-fixtures/checkpoint-v8.json.gz",
)));

function currentSave(version = 8, seed = "0", tick = "0"): Buffer {
  return Buffer.from(checkpoint.toString("utf8")
    .replace('"version":8,', `"version":${version},`)
    .replace('"world_seed":0,', `"world_seed":${seed},`)
    .replace('"tick_count":0,"next_animal_id"', `"tick_count":${tick},"next_animal_id"`));
}

function legacySave(version: number, seed: number): Buffer {
  return Buffer.from(JSON.stringify({
    version, world_seed: seed, tick_count: 0, organisms: [], animals: [],
    grid: { tiles: new Array(600 * 300).fill(0) },
  }));
}

function writeWorld(root: string, bytes: Buffer, hash = "source-world"): void {
  fs.mkdirSync(path.join(root, "worlds", hash), { recursive: true });
  fs.writeFileSync(path.join(root, "worlds", hash, "world.save"), bytes);
  fs.writeFileSync(path.join(root, "worlds", "_live"), hash);
}

function temporaryFolder(): string {
  return fs.mkdtempSync(path.join(tmpdir(), "thb-checkpoint-compatibility-"));
}

test("v8 Rust checkpoints survive synchronous, worker and download validation byte for byte", async () => {
  // JSON.parse/JSON.stringify would round both these RNG integers. Transport must not.
  assert.ok(checkpoint.includes('"stream":18446744073709551615'));
  assert.ok(checkpoint.includes('"word_pos":36893488147419103239'));
  const results = [
    safety.validateWorldSaveBytes(checkpoint),
    await safety.validateWorldSaveBytesInWorker(checkpoint),
    await safety.downloadAndValidateWorldSave("https://example.test/world.save", {
      fetchImpl: async () => new Response(new Uint8Array(checkpoint)),
    }),
  ];
  for (const result of results) {
    assert.equal(result.version, 8);
    assert.equal(result.seedText, "0");
    assert.equal(result.tickText, "0");
    assert.deepEqual(result.bytes, checkpoint);
  }
});

test("future schemas are refused by synchronous, worker and download validation", async () => {
  const future = currentSave(9);
  assert.throws(() => safety.validateWorldSaveBytes(future), /schema v9.*v8/);
  await assert.rejects(safety.validateWorldSaveBytesInWorker(future), /schema v9.*v8/);
  await assert.rejects(safety.downloadAndValidateWorldSave("https://example.test/world.save", {
    fetchImpl: async () => new Response(new Uint8Array(future)),
  }), /schema v9.*v8/);
});

test("only legacy schemas can carry the old grid dimensions", () => {
  for (let version = 0; version <= 7; version++) {
    assert.equal(safety.validateWorldSaveBytes(legacySave(version, 99)).version, version);
  }
  assert.throws(() => safety.validateWorldSaveBytes(legacySave(8, 0)), /expected 320000/);
});

test("seed zero has stable identity from schema 1 onward, including legacy imports", () => {
  const loaded = safety.validateWorldSaveBytes(checkpoint);
  for (let version = 1; version <= 8; version++) {
    const source = safety.validateWorldSaveBytes(version === 8 ? checkpoint : legacySave(version, 0));
    assert.doesNotThrow(() => safety.assertSameLoadedWorld(source, loaded));
  }
  assert.throws(() => safety.assertSameLoadedWorld(
    safety.validateWorldSaveBytes(legacySave(0, 0)), loaded,
  ), /no stable world seed/);
  assert.doesNotThrow(() => safety.assertSameLoadedWorld(
    safety.validateWorldSaveBytes(legacySave(0, 99)),
    safety.validateWorldSaveBytes(currentSave(8, "99")),
  ));
  assert.throws(() => safety.assertSameLoadedWorld(
    loaded, safety.validateWorldSaveBytes(currentSave(8, "1")),
  ), /expected imported seed 0/);
  const later = safety.validateWorldSaveBytes(currentSave(8, "0", "1"));
  assert.equal(later.tickText, "1");
  assert.throws(() => safety.assertSameLoadedWorld(later, loaded), /before imported tick/);
});

test("v8 checkpoints identify both existing worlds folders and root-level save files", () => {
  for (const rootLevel of [false, true]) {
    const root = temporaryFolder();
    try {
      if (rootLevel) fs.writeFileSync(path.join(root, "world.save"), checkpoint);
      else writeWorld(root, checkpoint);
      assert.ok(safety.requireOrUpgradeDataRootIdentity(root).id);
      const saved = rootLevel ? path.join(root, "world.save") : path.join(root, "worlds/source-world/world.save");
      assert.deepEqual(fs.readFileSync(saved), checkpoint);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  }
});

test("folder inspection accepts zero seeds in v7/v8 and refuses ambiguous schema-0 zero", async () => {
  const root = temporaryFolder();
  try {
    for (const bytes of [legacySave(7, 0), checkpoint, legacySave(0, 99)]) {
      writeWorld(root, bytes);
      const sync = safety.inspectSaveFolderMigrationSource(root);
      const worker = await safety.inspectSaveFolderMigrationSourceInWorker(root);
      assert.ok(sync);
      assert.ok(worker);
      assert.deepEqual(sync.bytes, bytes);
      assert.deepEqual(worker.bytes, bytes);
    }
    writeWorld(root, legacySave(0, 0));
    assert.throws(() => safety.inspectSaveFolderMigrationSource(root), /no stable seed/);
    await assert.rejects(safety.inspectSaveFolderMigrationSourceInWorker(root), /no stable seed/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

// Exercise the production IPC handlers, filesystem transactions and real workers.
// Only Electron, settings and the running Rust process/checkpoint endpoint are doubles.
function desktop(root: string) {
  const source = path.join(root, "source");
  const target = path.join(root, "target");
  const exported = path.join(root, "export.world.save");
  fs.mkdirSync(source);
  fs.mkdirSync(target);
  writeWorld(source, checkpoint);
  safety.initializeDataRootIdentity(source);
  let settings = { mode: "local", saveLocationOverride: source };
  const dataRoot = (s: typeof settings) => s.saveLocationOverride;
  const start = async (s: typeof settings, lock?: { release(): void }) => {
    if (!fs.existsSync(path.join(dataRoot(s), "worlds/_live"))) {
      writeWorld(dataRoot(s), checkpoint, "fresh-world");
    }
    lock?.release();
    return { port: 12345, commandToken: "test-token" };
  };
  const handlers: Record<string, (...args: unknown[]) => Promise<any>> = {};
  const context = vm.createContext({
    exports: {}, __dirname, process, console, setTimeout, clearTimeout, AbortController,
    fetch: async () => new Response(null, { status: 200 }),
    require: (id: string) => {
      if (id === "electron") return {
        dialog: {
          showSaveDialog: async () => ({ canceled: false, filePath: exported }),
          showMessageBox: async () => ({ response: 0 }),
        },
      };
      if (id === "./settings") return {
        loadSettings: () => settings,
        saveSettings: (s: typeof settings) => { settings = s; },
        defaultDataRoot: () => source,
        effectiveDataRoot: dataRoot,
        prepareDataRoot: dataRoot,
        worldsRoot: (s: typeof settings) => path.join(dataRoot(s), "worlds"),
      };
      if (id === "./sim-process") return {
        checkpointActiveSim: async () => true,
        stopSim: async () => {},
        startSim: start,
        startSimWithDataRootLock: start,
      };
      if (id === "./renderer-load") return { loadRenderer: async () => {} };
      return id.startsWith("./") ? require(path.join(__dirname, id)) : require(id);
    },
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, "ipc.js"), "utf8"), context);
  (context.exports as { registerIpc(ipc: unknown, window: () => object): void }).registerIpc({
    handle: (name: string, handler: (...args: unknown[]) => Promise<any>) => { handlers[name] = handler; },
  }, () => ({}));
  return { handlers, source, target, exported, settings: () => settings };
}

test("desktop export preserves all v8 checkpoint bytes", async () => {
  const root = temporaryFolder();
  try {
    const app = desktop(root);
    const result = await app.handlers["world:exportActive"]();
    assert.equal(result.exported, true);
    assert.deepEqual(fs.readFileSync(app.exported), checkpoint);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("desktop reset exports v8 and commits a fresh zero-seed world", async () => {
  const root = temporaryFolder();
  try {
    const app = desktop(root);
    const result = await app.handlers["world:resetLocal"]();
    assert.equal(result.reset, true);
    assert.equal(result.exported, true);
    assert.deepEqual(fs.readFileSync(app.exported), checkpoint);
    assert.equal(fs.readFileSync(path.join(app.source, "worlds/_live"), "utf8"), "fresh-world");
    assert.deepEqual(fs.readFileSync(path.join(app.source, "worlds/fresh-world/world.save")), checkpoint);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("desktop folder migration commits v8/zero through workers and the loaded-world handshake", async () => {
  const root = temporaryFolder();
  try {
    const app = desktop(root);
    const sidecar = Buffer.from([0, 255, 42]);
    fs.writeFileSync(path.join(app.source, "worlds/source-world/memory.sqlite"), sidecar);
    const result = await app.handlers["world:migrateDataRoot"]({}, { targetDir: app.target });
    assert.equal(result.migrated, true);
    assert.equal(app.settings().saveLocationOverride, app.target);
    for (const folder of [app.source, app.target]) {
      assert.deepEqual(fs.readFileSync(path.join(folder, "worlds/source-world/world.save")), checkpoint);
      assert.deepEqual(fs.readFileSync(path.join(folder, "worlds/source-world/memory.sqlite")), sidecar);
    }
    const expected = safety.inspectSaveFolderMigrationSource(app.source);
    safety.verifySaveFolderMigrationCopy(app.target, expected);
    await safety.verifySaveFolderMigrationCopyInWorker(app.target, expected);
    assert.equal(safety.hasRecoverableSaveFolderMigration(app.source, app.target), false);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
