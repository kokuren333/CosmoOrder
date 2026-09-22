// Navigation groups only what the package declares. These tests use the real
// Golden lesson shape so a change in the grouping shows up immediately.

import test from "node:test";
import assert from "node:assert/strict";
import { outline, readingOrder, search } from "../src/outline.ts";
import type { LessonView } from "../src/types.ts";

function lesson(): LessonView {
  return {
    package_id: "org.example/arithmetic",
    package_version: "0.1.0",
    digest: "a".repeat(64),
    manifest: {
      schema_version: "0.1",
      package_id: "org.example/arithmetic",
      package_version: "0.1.0",
      title: "算数の基礎",
      language: "ja-JP",
      capabilities: { required: [], optional: [] },
      entities: {},
      extensions: {},
    },
    concepts: [
      { id: "addition", title: "足し算", requires: [] },
      { id: "subtraction", title: "引き算", requires: ["addition"] },
    ],
    objectives: [
      { id: "addition.basic", concept: "addition", description: "小さな整数の足し算ができる" },
      { id: "subtraction.basic", concept: "subtraction", description: "小さな整数の引き算ができる" },
    ],
    curricula: [
      { id: "intro", title: "はじめての算数", objectives: ["addition.basic", "subtraction.basic"] },
    ],
    resources: [
      { id: "addition.lesson", type: "markdown", title: "1と1を合わせる", path: "content/a.md", teaches: ["addition.basic"] },
      { id: "subtraction.lesson", type: "markdown", title: "1を引く", path: "content/b.md", teaches: ["subtraction.basic"] },
    ],
    assessments: [
      {
        id: "addition.01",
        revision: "1",
        measures: ["addition.basic"],
        stimulus: { markdown: "1 + 1 はいくつ？" },
        response: { type: "single_select", options: [{ id: "a", text: "1" }, { id: "b", text: "2" }] },
        evaluation: { type: "exact", answer: "b" },
        feedback: { markdown: "1に1を足すと2です。" },
      },
      {
        id: "subtraction.01",
        revision: "1",
        measures: ["subtraction.basic"],
        stimulus: { markdown: "2 - 1 は 1 ですか？" },
        response: { type: "boolean" },
        evaluation: { type: "exact", answer: true },
        feedback: { markdown: "2から1を引くと1です。" },
      },
    ],
    stimuli: {},
  };
}

test("curricula group objectives under their concept", () => {
  const view = outline(lesson());
  assert.equal(view.curricula.length, 1);
  const curriculum = view.curricula[0];
  assert.ok(curriculum !== undefined);
  assert.equal(curriculum.curriculum.id, "intro");
  assert.deepEqual(
    curriculum.concepts.map((node) => node.concept.id),
    ["addition", "subtraction"],
  );
  const addition = curriculum.concepts[0];
  assert.ok(addition !== undefined);
  assert.deepEqual(addition.objectives.map((objective) => objective.id), ["addition.basic"]);
  assert.deepEqual(addition.resources.map((resource) => resource.id), ["addition.lesson"]);
  assert.deepEqual(addition.assessments.map((assessment) => assessment.id), ["addition.01"]);
});

test("a concept no curriculum lists stays visible", () => {
  const data = lesson();
  const view = outline({
    ...data,
    concepts: [...data.concepts, { id: "multiplication", title: "掛け算", requires: [] }],
  });
  assert.deepEqual(
    view.unlisted_concepts.map((node) => node.concept.id),
    ["multiplication"],
  );
});

test("reading order follows the curriculum and then the package", () => {
  const order = readingOrder(lesson());
  assert.deepEqual(
    order.map((resource) => resource.id),
    ["addition.lesson", "subtraction.lesson"],
  );
});

test("search matches identifiers and titles only", () => {
  const data = lesson();
  assert.deepEqual(
    search(data, "引き算").concepts.map((concept) => concept.id),
    ["subtraction"],
  );
  assert.deepEqual(
    search(data, "addition.01").assessments.map((assessment) => assessment.id),
    ["addition.01"],
  );
  assert.deepEqual(search(data, "   ").resources, []);
});
