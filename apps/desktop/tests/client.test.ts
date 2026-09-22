// The IPC boundary must keep the diagnostics Core and Package produced, and
// must never turn a rejection into an untyped failure.

import test from "node:test";
import assert from "node:assert/strict";
import { OsmiumError, toOsmiumError } from "../src/client.ts";

test("a structured rejection keeps every diagnostic", () => {
  const error = toOsmiumError({
    diagnostics: [
      {
        code: "OSM_REFERENCE",
        message: "objective references a missing concept",
        file: "entities/objectives.json",
        path: "/0/concept",
        suggestions: ["declare the concept first"],
      },
      {
        code: "OSM_PATH",
        message: "path escapes the package root",
        file: null,
        path: "",
        suggestions: [],
      },
    ],
  });
  assert.ok(error instanceof OsmiumError);
  assert.equal(error.diagnostics.length, 2);
  assert.equal(error.diagnostics[0]?.code, "OSM_REFERENCE");
  assert.equal(error.diagnostics[0]?.suggestions.length, 1);
  assert.ok(error.message.includes("OSM_REFERENCE: objective references a missing concept"));
});

test("an unstructured rejection becomes a shell diagnostic", () => {
  const error = toOsmiumError(new Error("the window is closed"));
  assert.equal(error.diagnostics.length, 1);
  assert.equal(error.diagnostics[0]?.code, "OSM_SHELL");
  assert.equal(error.diagnostics[0]?.message, "the window is closed");
});

test("a string rejection is reported verbatim", () => {
  const error = toOsmiumError("denied by the capability set");
  assert.equal(error.diagnostics[0]?.message, "denied by the capability set");
});

test("an already normalized error is returned unchanged", () => {
  const first = toOsmiumError({ diagnostics: [{ code: "OSM_IO", message: "no", file: null, path: "", suggestions: [] }] });
  assert.equal(toOsmiumError(first), first);
});
