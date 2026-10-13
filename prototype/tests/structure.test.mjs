import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const projectRoot = path.join(import.meta.dirname, "..");
const roots = ["src", path.join("src-tauri", "src"), "scripts"].map((entry) => path.join(projectRoot, entry));
const extensions = new Set([".rs", ".ts", ".tsx", ".css", ".js", ".mjs"]);
const ignored = new Set(["gen", "target", "dist"]);

async function sourceFiles(directory) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (ignored.has(entry.name)) continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await sourceFiles(entryPath));
    else if (extensions.has(path.extname(entry.name))) result.push(entryPath);
  }
  return result;
}

test("source modules stay below the review size limit", async () => {
  const files = (await Promise.all(roots.map(sourceFiles))).flat();
  const large = [];
  for (const file of files) {
    const lines = (await readFile(file, "utf8")).split("\n").length;
    if (lines > 400) large.push(`${path.relative(projectRoot, file)} (${lines})`);
  }
  assert.deepEqual(large, []);
});
