import { test } from "node:test";
import assert from "node:assert/strict";
import { attemptsFor, defaultObjectiveId, objectiveForConcept, routePosition } from "../src/learningNavigation.ts";
import type { LessonView, ObjectiveProgress } from "../src/types.ts";

const lesson = {
  package_id: "example/math", package_version: "1.0", digest: "digest",
  manifest: { schema_version: "0.1", package_id: "example/math", package_version: "1.0", title: "Math", language: "en", capabilities: { required: [], optional: [] }, entities: {}, extensions: {} },
  concepts: [
    { id: "limit", title: "Limits", requires: [] },
    { id: "derivative", title: "Derivatives", requires: ["limit"] },
    { id: "optimization", title: "Optimization", requires: ["derivative"] },
  ],
  objectives: [
    { id: "limit.o", concept: "limit", description: "Understand limits" },
    { id: "derivative.o", concept: "derivative", description: "Differentiate" },
    { id: "optimization.o", concept: "optimization", description: "Optimize" },
  ],
  curricula: [{ id: "path", title: "Calculus", objectives: ["limit.o", "derivative.o", "optimization.o"] }],
  resources: [], assessments: [], stimuli: {},
} as LessonView;

test("Route uses explicit Curriculum order separately from Concept prerequisites", () => {
  assert.equal(defaultObjectiveId(lesson), "limit.o");
  assert.equal(objectiveForConcept(lesson, "derivative"), "derivative.o");
  const route = routePosition(lesson, "derivative.o");
  assert.equal(route.index, 1);
  assert.equal(route.before?.id, "limit.o");
  assert.equal(route.after?.id, "optimization.o");
  assert.deepEqual(route.prerequisiteConcepts.map((item) => item.id), ["limit"]);
  assert.deepEqual(route.dependentConcepts.map((item) => item.id), ["optimization"]);
});

test("Route does not infer a sequence when a Curriculum is absent", () => {
  const withoutCurriculum = { ...lesson, curricula: [] };
  const route = routePosition(withoutCurriculum, "derivative.o");
  assert.equal(route.curriculum, null);
  assert.deepEqual(route.orderedObjectives, []);
  assert.equal(route.before, null);
  assert.equal(route.after, null);
  assert.deepEqual(route.prerequisiteConcepts.map((item) => item.id), ["limit"]);
});

test("learning navigation reports observed attempts only", () => {
  const progress: ObjectiveProgress[] = [{ objective_id: "derivative.o", attempts: 2, correct: 1, accuracy: 0.5, last_score: 1, last_timestamp: null }];
  assert.equal(attemptsFor(progress, "derivative.o"), 2);
  assert.equal(attemptsFor(progress, "limit.o"), 0);
});
