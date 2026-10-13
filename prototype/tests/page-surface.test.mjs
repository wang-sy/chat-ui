import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const root = path.join(import.meta.dirname, "..");

async function source(relativePath) {
  return readFile(path.join(root, relativePath), "utf8");
}

test("Space uses one page surface instead of a competing canvas graph", async () => {
  const [app, surface, sidebar, styles, vite] = await Promise.all([
    source("src/App.tsx"),
    source("src/components/MainPageSurface.tsx"),
    source("src/components/Sidebar.tsx"),
    source("src/styles/space-surface.css"),
    source("vite.config.mjs"),
  ]);
  assert.match(app, /MainPageSurface/);
  assert.doesNotMatch(app, /SpaceCanvas|ReactFlow/);
  assert.match(surface, /main-page-surface/);
  assert.match(surface, /Main Thread 页面/);
  assert.doesNotMatch(surface, /ReactFlow|space-flow|NodeResizer/);
  assert.doesNotMatch(sidebar, /主画板|CaretDownIcon/);
  assert.match(styles, /\.space-surface/);
  assert.doesNotMatch(styles, /react-flow|space-flow|canvas-empty/);
  assert.doesNotMatch(vite, /@xyflow\/react|flow:/);
});
