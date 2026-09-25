import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";
import type { ViteDevServer } from "vite";

let server: ViteDevServer;
before(async () => {
  server = await createServer({
    server: { middlewareMode: true },
    appType: "custom",
  });
});
after(async () => {
  await server?.close();
});
const noop = () => {};

test("technical metadata is present in a closed native disclosure", async () => {
  const { PackageList } = (await server.ssrLoadModule(
    "/src/components/Packages.tsx",
  )) as typeof import("../src/components/Packages.tsx");
  const html = renderToStaticMarkup(
    createElement(PackageList, {
      packages: [
        {
          package_id: "private-id",
          title: "教材の名前",
          entity_counts: { concepts: 5, objectives: 9, resources: 4, assessments: 12, curricula: 1 },
          package_version: "1.0",
          schema_version: "0.1",
          digest: "private-digest",
          selected_version: "1.0",
        },
      ],
      selected: null,
      busy: false,
      loadState: "ready",
      loadError: [],
      onSelect: noop,
      onRefresh: noop,
      onCreate: noop,
      onOpenSource: noop,
      onImport: noop,
      onRemove: noop,
    }),
  );
  assert.match(html, /<h2>教材の名前<\/h2>/);
  assert.match(html, /<dt>テーマ<\/dt><dd>5<\/dd>/);
  assert.match(html, /<dt>問題<\/dt><dd>12<\/dd>/);
  assert.match(
    html,
    /<details class="developer-details"><summary>技術情報<\/summary>.*private-id.*private-digest/s,
  );
  assert.doesNotMatch(html, /<details[^>]*\bopen\b/);
});

test("lesson exposes package hierarchy with absent optional metadata", async () => {
  const { Lesson } = (await server.ssrLoadModule(
    "/src/components/Lesson.tsx",
  )) as typeof import("../src/components/Lesson.tsx");
  const lesson = {
    package_id: "org.example/sample", package_version: "0.1.0", digest: "abc",
    manifest: { schema_version: "0.1", package_id: "org.example/sample", package_version: "0.1.0", title: "Sample", language: "en-US", capabilities: { required: [], optional: [] }, entities: {}, extensions: {} },
    concepts: [
      { id: "objects", title: "Objects", requires: [] },
      { id: "count", title: "Counting", requires: ["objects"] },
    ],
    objectives: [{ id: "count.basic", concept: "count", description: "Count objects" }],
    curricula: [{ id: "intro", title: "Intro", objectives: ["count.basic"] }],
    resources: [{ id: "lesson", type: "markdown", title: "Read counting", path: "content/a.md", teaches: ["count.basic"] }],
    assessments: [{ id: "check", revision: "1", measures: ["count.basic"], stimulus: { markdown: "How many?" }, response: { type: "boolean" as const }, evaluation: { type: "exact", answer: true }, feedback: { markdown: "Yes" } }],
    stimuli: { check: { text: "How many?", content: { blocks: [] } } },
  };
  const html = renderToStaticMarkup(createElement(Lesson, {
    lesson, progress: [], busy: false, currentObjectiveId: "count.basic", onOpenResource: noop, onOpenAssessment: noop, onOpenAtlas: noop, onOpenObjective: noop, onOpenRoute: noop,
  }));
  assert.match(html, /<h3>Counting<\/h3>/);
  assert.match(html, /<p class="eyebrow">学習目標<\/p>/);
  assert.match(html, /問題 · 確かめる/);
  assert.match(html, /<h1 id="lesson-heading">Sample<\/h1>/);
  assert.match(html, /前提を見る:<\/span><button[^>]*>Objects<\/button>/);
  assert.match(html, /Read counting/);
  assert.match(html, /How many\?/);
  assert.match(html, /aria-label="カリキュラムの現在位置"/);
  assert.match(html, /Schema<\/dt><dd>0\.1<\/dd>/);
});

test("Lesson keeps a no-assessment objective honest and collapses later modules", async () => {
  const { Lesson } = (await server.ssrLoadModule("/src/components/Lesson.tsx")) as typeof import("../src/components/Lesson.tsx");
  const lesson = {
    package_id: "example/tiny", package_version: "1.0", digest: "digest",
    manifest: { schema_version: "0.1", package_id: "example/tiny", package_version: "1.0", title: "Tiny", language: "ja", capabilities: { required: [], optional: [] }, entities: {}, extensions: {} },
    concepts: [{ id: "one", title: "One", requires: [] }, { id: "two", title: "Two", requires: [] }],
    objectives: [{ id: "one.o", concept: "one", description: "First" }, { id: "two.o", concept: "two", description: "Second" }],
    curricula: [{ id: "first", title: "First module", objectives: ["one.o"] }, { id: "second", title: "Second module", objectives: ["two.o"] }],
    resources: [{ id: "shared", type: "markdown", title: "Shared", path: "content/a.md", teaches: ["one.o", "two.o"] }],
    assessments: [], stimuli: {},
  };
  const html = renderToStaticMarkup(createElement(Lesson, { lesson, progress: [], busy: false, currentObjectiveId: null, onOpenResource: noop, onOpenAssessment: noop, onOpenAtlas: noop, onOpenObjective: noop, onOpenRoute: noop }));
  assert.match(html, /<details class="curriculum curriculum-section" open="">/);
  assert.match(html, /<details class="curriculum curriculum-section">/);
  assert.equal((html.match(/aria-label="教材を開く:Shared"/g) ?? []).length, 1);
  assert.doesNotMatch(html, /回答記録なし|No answers recorded/);
  assert.doesNotMatch(html, /<h3>Two<\/h3>/);
  const sparseHtml = renderToStaticMarkup(createElement(Lesson, {
    lesson: { ...lesson, concepts: [lesson.concepts[0]!], objectives: [lesson.objectives[0]!], curricula: [] },
    progress: [], busy: false, currentObjectiveId: null, onOpenResource: noop, onOpenAssessment: noop, onOpenAtlas: noop, onOpenObjective: noop, onOpenRoute: noop,
  }));
  assert.match(sparseHtml, /<h2>カリキュラム外のテーマ<\/h2>/);
  assert.match(sparseHtml, /aria-label="教材を開く:Shared"/);
  assert.doesNotMatch(sparseHtml, /lesson-current-route/);
});

test("pressure-volume prototype renders native controls and an inert-resource fallback", async () => {
  const { PressureVolumePrototype } = (await server.ssrLoadModule("/src/components/PressureVolumePrototype.tsx")) as typeof import("../src/components/PressureVolumePrototype.tsx");
  const html = renderToStaticMarkup(createElement(PressureVolumePrototype));
  assert.match(html, /role="group" aria-label="模式図の条件"/);
  assert.match(html, /type="radio" name="pv-answer"/);
  assert.match(html, /role="img" aria-label="模式的な圧・容積ループ/);
  assert.match(html, /操作欄に問題がある場合も、上の教材本文から学習を続けられます/);
  assert.doesNotMatch(html, /<script\b|dangerouslySetInnerHTML|eval\(/i);
});

const atlasLesson = {
  package_id: "example/math", package_version: "1.0", digest: "digest",
  manifest: { schema_version: "0.1", package_id: "example/math", package_version: "1.0", title: "Math", language: "en", capabilities: { required: [], optional: [] }, entities: {}, extensions: {} },
  concepts: [{ id: "limit", title: "Limits", requires: [] }, { id: "derivative", title: "Derivatives", requires: ["limit"] }, { id: "integral", title: "Integrals", requires: ["derivative"] }],
  objectives: [
    { id: "limit.o", concept: "limit", description: "Understand limits" },
    { id: "derivative.o", concept: "derivative", description: "Differentiate a function" },
    { id: "integral.o", concept: "integral", description: "Integrate a function" },
  ],
  curricula: [{ id: "calculus", title: "Calculus", objectives: ["limit.o", "derivative.o", "integral.o"] }],
  resources: [{ id: "derivative.r", type: "markdown", title: "Derivative lesson", path: "content/derivative.md", teaches: ["derivative.o"] }],
  assessments: [{ id: "derivative.a", revision: "1", measures: ["derivative.o"], stimulus: { markdown: "Differentiate x²" }, response: { type: "boolean" as const }, evaluation: { type: "exact", answer: true }, feedback: { markdown: "Correct" } }],
  stimuli: {},
};

const atlasContext = {
  target: { id: "derivative", kind: "concept" as const, title: "Derivatives", detail: {} }, depth: 2, truncated: false, prerequisite_depth: 1, content_is_untrusted: true,
  nodes: [
    { id: "derivative", kind: "concept" as const, title: "Derivatives", depth: 0, entity: { requires: ["limit"] } },
    { id: "limit", kind: "concept" as const, title: "Limits", depth: 1, entity: { requires: [] } },
    { id: "integral", kind: "concept" as const, title: "Integrals", depth: 1, entity: { requires: ["derivative"] } },
    { id: "derivative.o", kind: "objective" as const, title: "Differentiate a function", depth: 1, entity: { concept: "derivative" } },
    { id: "derivative.r", kind: "resource" as const, title: "Derivative lesson", depth: 2, entity: { teaches: ["derivative.o"] } },
    { id: "derivative.a", kind: "assessment" as const, title: "derivative.a", depth: 2, entity: { measures: ["derivative.o"] } },
    { id: "calculus", kind: "curriculum" as const, title: "Calculus", depth: 2, entity: { objectives: ["limit.o", "derivative.o", "integral.o"] } },
  ],
  relations: [
    { relation: "requires", from_kind: "concept" as const, from_id: "derivative", to_kind: "concept" as const, to_id: "limit", incoming: false },
    { relation: "requires", from_kind: "concept" as const, from_id: "derivative", to_kind: "concept" as const, to_id: "integral", incoming: true },
  ],
};

test("Route presents author order and prerequisites as separate meanings", async () => {
  const { RoutePanel } = (await server.ssrLoadModule("/src/components/RoutePanel.tsx")) as typeof import("../src/components/RoutePanel.tsx");
  const html = renderToStaticMarkup(createElement(RoutePanel, {
    lesson: atlasLesson, objectiveId: "derivative.o", progress: [], busy: false,
    onSelectObjective: noop, onOpenAtlas: noop, onOpenResource: noop, onBack: noop,
  }));
  assert.match(html, /作者が定義した順序/);
  assert.match(html, /学習順序/);
  assert.match(html, /明示された前提関係/);
  assert.match(html, /Limits/);
  assert.match(html, /次は「/);
  assert.doesNotMatch(html, /mastery|mastered|mastered|習得済み|理解済み/i);
  assert.match(html, /aria-current="step"/);
});

test("Atlas renders a bounded local neighborhood and associated learning layers", async () => {
  const { AtlasPanel } = (await server.ssrLoadModule("/src/components/AtlasPanel.tsx")) as typeof import("../src/components/AtlasPanel.tsx");
  const html = renderToStaticMarkup(createElement(AtlasPanel, {
    lesson: atlasLesson, selectedConceptId: "derivative", context: atlasContext,
    progress: [], busy: false, onSearch: async () => ({ query: "", total: 0, limit: 20, results: [], truncated: false }),
    onRecenter: noop, onStudyConcept: noop, onOpenResource: noop, onOpenAssessment: noop,
    onBackToRoute: noop, onBackToLesson: noop,
  }));
  for (const text of ["Derivatives", "Limits", "Integrals", "Differentiate a function", "Derivative lesson", "カリキュラム上の位置", "2/3"]) assert.ok(html.includes(text), `${text}\n${html}`);
  assert.match(html, /点線は教材に定義された前提関係です/);
  assert.match(html, /このテーマを学ぶ/);
  assert.match(html, /Lessonへ戻る/);
  assert.doesNotMatch(html, /mastery|mastered|習得済み|理解済み/i);
});

test("Atlas handles a sparse Concept and caps visible neighbors", async () => {
  const { AtlasPanel } = (await server.ssrLoadModule("/src/components/AtlasPanel.tsx")) as typeof import("../src/components/AtlasPanel.tsx");
  const sparse = { ...atlasContext, nodes: [{ id: "derivative", kind: "concept" as const, title: "Derivatives", depth: 0, entity: { requires: [] } }], relations: [] };
  const props = {
    lesson: atlasLesson, selectedConceptId: "derivative", context: sparse,
    progress: [], busy: false, onSearch: async () => ({ query: "", total: 0, limit: 20, results: [], truncated: false }),
    onRecenter: noop, onStudyConcept: noop, onOpenResource: noop, onOpenAssessment: noop,
    onBackToRoute: noop, onBackToLesson: noop,
  };
  const sparseHtml = renderToStaticMarkup(createElement(AtlasPanel, props));
  assert.match(sparseHtml, /前提関係はまだ定義されていません/);
  const nodes = [sparse.nodes[0]!, ...Array.from({ length: 8 }, (_, index) => ({ id: `p${index}`, kind: "concept" as const, title: `Prerequisite ${index}`, depth: 1, entity: { requires: [] } }))];
  const bounded = { ...sparse, nodes, relations: nodes.slice(1).map((node) => ({ relation: "requires", from_kind: "concept" as const, from_id: "derivative", to_kind: "concept" as const, to_id: node.id, incoming: false })) };
  const boundedHtml = renderToStaticMarkup(createElement(AtlasPanel, { ...props, context: bounded }));
  assert.match(boundedHtml, /他2件は近傍表示の上限外です/);
  assert.equal((boundedHtml.match(/class="atlas-neighbor"/g) ?? []).length, 6);
});

test("authoring projection preserves complete reference metadata and diagnostic severity", async () => {
  const { AuthoringReview } = (await server.ssrLoadModule(
    "/src/components/AuthoringReview.tsx",
  )) as typeof import("../src/components/AuthoringReview.tsx");
  const html = renderToStaticMarkup(createElement(AuthoringReview, {
    references: [{
      id: "internal-guideline",
      kind: "url",
      type: "guideline",
      title: "Internal guideline",
      visibility: "attribution_only",
      record_visibility: "public",
      locator_visibility: "hidden",
      locator: "https://intranet.example.org/guideline",
      citation: "Example Organization. Internal guideline.",
      authors: ["A. Author", "B. Author"],
      publisher: "Example Organization",
      published_at: "2024-02",
      identifiers: { doi: "10.0000/example" },
      content_hash: `sha256:${"a".repeat(64)}`,
    }],
    diagnostics: [
      {
        code: "OSM_LINT_LICENSE_UNKNOWN", severity: "warning",
        message: "license status is unknown", file: "entities/resources.json",
        line: null, column: null,
        path: "/0/license", suggestions: ["confirm reuse conditions"],
        entity_type: "resource", entity_id: "lesson",
      },
      {
        code: "OSM_REFERENCE", severity: "error",
        message: "reference ID does not exist", file: "entities/resources.json",
        line: null, column: null,
        path: "/0/evidence_reference_ids/0", suggestions: [],
      },
    ],
  }));
  assert.match(html, /Internal guideline/);
  assert.match(html, /data-field="locator">https:\/\/intranet\.example\.org\/guideline/);
  assert.match(html, /data-field="identifiers">\{&quot;doi&quot;:&quot;10\.0000\/example&quot;\}/);
  assert.match(html, /data-field="authors">A\. Author, B\. Author/);
  assert.match(html, /data-severity="warning"/);
  assert.match(html, /data-severity="error"/);
  assert.match(html, /entities\/resources\.json · \/0\/license/);
  assert.match(html, /confirm reuse conditions/);
});

test("resource references show public and attribution metadata without navigation or private leakage", async () => {
  const { ResourceReferences } = (await server.ssrLoadModule(
    "/src/components/ResourceReferences.tsx",
  )) as typeof import("../src/components/ResourceReferences.tsx");
  const references = [
    {
      id: "public",
      title: "公開資料",
      visibility: "public" as const,
      record_visibility: "public" as const,
      locator_visibility: "public" as const,
      citation: "公的機関. 資料名。",
      locator: "https://example.org/public",
      authors: ["A. Author"],
      publisher: "Example Organization",
      version: "2",
    },
    {
      id: "credit",
      title: "謝辞のみの資料",
      visibility: "attribution_only" as const,
      record_visibility: "public" as const,
      locator_visibility: "hidden" as const,
      citation: "発行元. 指針名。",
      locator: "https://example.org/must-not-display",
    },
    {
      id: "private",
      title: "PRIVATE_SENTINEL",
      visibility: "private" as const,
      record_visibility: "private" as const,
      citation: "private citation",
      locator: "C:/private/file.pdf",
    },
  ] as unknown as import("../src/types.ts").ResourceReference[];
  const html = renderToStaticMarkup(createElement(ResourceReferences, { references }));
  assert.match(html, /<summary>参考資料<\/summary>/);
  assert.match(html, /公開資料/);
  assert.match(html, /公的機関\. 資料名。/);
  assert.match(html, /https:\/\/example\.org\/public/);
  assert.match(html, /A\. Author · Example Organization · 2/);
  assert.match(html, /謝辞のみの資料/);
  assert.match(html, /発行元\. 指針名。/);
  assert.doesNotMatch(html, /must-not-display|PRIVATE_SENTINEL|C:\/private/);
  assert.doesNotMatch(html, /<a\b/);
  assert.equal(renderToStaticMarkup(createElement(ResourceReferences, { references: [] })), "");
});

test("locator visibility is decided independently of record visibility", async () => {
  const { ResourceReferences } = (await server.ssrLoadModule(
    "/src/components/ResourceReferences.tsx",
  )) as typeof import("../src/components/ResourceReferences.tsx");
  // A record can be attributable while its locator stays hidden, and the split
  // fields win over the legacy enum when both are present.
  const contradictory = [
    {
      id: "split-wins",
      title: "分割フィールドを優先する資料",
      visibility: "public" as const,
      record_visibility: "public" as const,
      locator_visibility: "hidden" as const,
      citation: "発行元. 資料名。",
      locator: "https://example.org/hidden-by-split",
    },
    {
      id: "legacy-fallback",
      title: "旧形式の謝辞のみ資料",
      visibility: "attribution_only" as const,
      citation: "発行元. 旧形式。",
      locator: "https://example.org/hidden-by-legacy",
    },
  ] as unknown as import("../src/types.ts").ResourceReference[];
  const html = renderToStaticMarkup(createElement(ResourceReferences, { references: contradictory }));
  assert.match(html, /分割フィールドを優先する資料/);
  assert.match(html, /旧形式の謝辞のみ資料/);
  assert.doesNotMatch(html, /hidden-by-split|hidden-by-legacy/);
  assert.equal((html.match(/data-locator-visibility="hidden"/g) ?? []).length, 2);
});

test("empty library still offers the install path", async () => {
  const { PackageList } = (await server.ssrLoadModule(
    "/src/components/Packages.tsx",
  )) as typeof import("../src/components/Packages.tsx");
  const html = renderToStaticMarkup(createElement(PackageList, {
    packages: [], selected: null, busy: false, loadState: "ready", loadError: [], onSelect: noop, onRefresh: noop, onCreate: noop, onOpenSource: noop, onImport: noop, onRemove: noop,
  }));
  assert.match(html, /教材がありません/);
  assert.match(html, /教材を追加/);
  assert.match(html, /osmium install/);
  assert.match(html, /osmium install/);
});

test("library loading state never looks like an empty library", async () => {
  const { PackageList } = (await server.ssrLoadModule(
    "/src/components/Packages.tsx",
  )) as typeof import("../src/components/Packages.tsx");
  const html = renderToStaticMarkup(createElement(PackageList, {
    packages: [], selected: null, busy: true, loadState: "loading", loadError: [],
    onSelect: noop, onRefresh: noop, onCreate: noop, onOpenSource: noop, onImport: noop, onRemove: noop,
  }));
  assert.match(html, /読み込み中/);
  assert.doesNotMatch(html, /教材がありません/);
});

test("library error state shows diagnostics and a retry action", async () => {
  const { PackageList } = (await server.ssrLoadModule(
    "/src/components/Packages.tsx",
  )) as typeof import("../src/components/Packages.tsx");
  const html = renderToStaticMarkup(createElement(PackageList, {
    packages: [], selected: null, busy: false, loadState: "error",
    loadError: [{ code: "OSM_IO", severity: "error", message: "library unavailable", file: null, line: null, column: null, path: "", suggestions: [] }],
    onSelect: noop, onRefresh: noop, onCreate: noop, onOpenSource: noop, onImport: noop, onRemove: noop,
  }));
  assert.match(html, /ライブラリを読み込めませんでした/);
  assert.match(html, /OSM_IO/);
  assert.match(html, /再試行/);
  assert.doesNotMatch(html, /教材がありません/);
});

test("version cards carry exact package identity and damaged packages can only be removed", async () => {
  const { PackageList } = (await server.ssrLoadModule(
    "/src/components/Packages.tsx",
  )) as typeof import("../src/components/Packages.tsx");
  const html = renderToStaticMarkup(createElement(PackageList, {
    packages: [
      { package_id: "same/id", package_version: "1.0.0", schema_version: "0.1", title: "First", digest: "a", selected_version: "1.0.0", entity_counts: {}, integrity_error: null },
      { package_id: "same/id", package_version: "2.0.0", schema_version: "0.1", title: "Second", digest: "b", selected_version: "2.0.0", entity_counts: {}, integrity_error: null },
      { package_id: "damaged/id", package_version: "1.0.0", schema_version: "0.1", title: "Damaged", digest: "c", selected_version: "1.0.0", entity_counts: {}, integrity_error: "OSM_HASH: damaged" },
    ],
    selected: null, busy: false, loadState: "ready", loadError: [],
    onSelect: noop, onRefresh: noop, onCreate: noop, onOpenSource: noop, onImport: noop, onRemove: noop,
  }));
  assert.match(html, /data-package-id="same\/id" data-package-version="1\.0\.0"/);
  assert.match(html, /data-package-id="same\/id" data-package-version="2\.0\.0"/);
  assert.match(html, /data-package-id="damaged\/id"[^>]*disabled=""/);
  assert.match(html, /この教材ファイルは読み込めません/);
  assert.equal((html.match(/class="ghost package-remove"/g) ?? []).length, 3);
});

test("shell marks the current learning context and disables course navigation without a lesson", async () => {
  const { AppShell } = (await server.ssrLoadModule(
    "/src/components/AppShell.tsx",
  )) as typeof import("../src/components/AppShell.tsx");
  const props = {
    section: "reader",
    title: "算数",
    scale: 2,
    busy: false,
    status: null,
    onScale: noop,
    onLibrary: noop,
    onLesson: noop,
    onProgress: noop,
    onHistory: noop,
    onAuthoring: noop,
    language: "ja" as const,
    onLanguage: noop,
    children: "本文",
  };
  const html = renderToStaticMarkup(createElement(AppShell, props));
  assert.match(html, /aria-label="目次を表示" aria-current="page"/);
  assert.equal((html.match(/aria-current="page"/g) ?? []).length, 1);
  assert.match(html, /aria-pressed="true">200%/);
  const english = renderToStaticMarkup(createElement(AppShell, { ...props, language: "en" as const }));
  assert.match(english, /Skip to content/);
  assert.match(english, /Library/);
  assert.match(english, /Stored on this device/);
  const empty = renderToStaticMarkup(
    createElement(AppShell, { ...props, section: "packages", title: null }),
  );
  assert.match(empty, /aria-label="進捗" disabled=""/);
  assert.match(empty, /aria-label="履歴"[^>]*>/);
  assert.doesNotMatch(empty.match(/<button[^>]*aria-label="履歴"[^>]*>/)?.[0] ?? "", /disabled/);
  assert.match(empty, /CosmoOrder/);
});

test("orphan history remains readable without a package payload", async () => {
  const { HistoryPanel } = (await server.ssrLoadModule(
    "/src/components/HistoryPanel.tsx",
  )) as typeof import("../src/components/HistoryPanel.tsx");
  const html = renderToStaticMarkup(createElement(HistoryPanel, {
    events: [{
      event_id: "event-1", event_type: "assessment_attempt",
      package_id: "org.example/course", package_version: "1.0.0",
      assessment_id: "check", assessment_revision: "1", objective_ids: ["objective.local"],
      concept_ids: [], response: true, score: 1, correct: true,
      timestamp: "2026-09-25T00:00:00Z", duration_ms: null, hints_used: null,
      evaluator: { id: "exact", version: "1" },
      assessment_snapshot: { stimulus: { markdown: "保存された問題文" } },
    }],
    objectives: new Map(), stimuli: {}, showPackageIdentity: true,
    onLoadMore: noop, onBack: noop, busy: false,
  }));
  assert.match(html, /保存された問題文/);
  assert.match(html, /org\.example\/course@1\.0\.0/);
  assert.match(html, /objective\.local/);
  assert.match(html, /正解/);
});

test("progress renders observed accuracy and hides maintenance; missing accuracy stays missing", async () => {
  const { ProgressPanel } = (await server.ssrLoadModule(
    "/src/components/ProgressPanel.tsx",
  )) as typeof import("../src/components/ProgressPanel.tsx");
  const html = renderToStaticMarkup(
    createElement(ProgressPanel, {
      progress: [
        {
          objective_id: "internal",
          attempts: 4,
          correct: 3,
          accuracy: 0.75,
          last_score: 1,
          last_timestamp: null,
        },
        {
          objective_id: "untried",
          attempts: 0,
          correct: 0,
          accuracy: null,
          last_score: null,
          last_timestamp: null,
        },
      ],
      objectives: new Map([["internal", "足し算ができる"]]),
      onRebuild: noop,
      onBack: noop,
      busy: false,
    }),
  );
  assert.match(html, /75%/);
  assert.match(html, /3\/4 正答/);
  assert.match(html, /まだ回答なし/);
  assert.match(html, /回答数（延べ）/);
  assert.equal((html.match(/<meter /g) ?? []).length, 1);
  assert.match(
    html,
    /<details class="developer-details"><summary>メンテナンス<\/summary>.*イベントから再構築/s,
  );
  assert.doesNotMatch(html, /習得率|習得済み/);
});

test("both assessment types use labeled radio cards and require explicit grading", async () => {
  const { AssessmentView } = (await server.ssrLoadModule(
    "/src/components/Assessment.tsx",
  )) as typeof import("../src/components/Assessment.tsx");
  const assessment = {
    id: "question-id",
    revision: "1",
    measures: ["objective-id"],
    stimulus: { markdown: "問題" },
    response: { type: "boolean" as const },
    evaluation: { type: "exact", answer: true },
    feedback: { markdown: "解説" },
  };
  const props = {
    assessment,
    stimulus: { text: "問題", content: { blocks: [] } },
    attempt: null,
    busy: false,
    onAnswer: noop,
    onBack: noop,
    onNext: noop,
    onShowProgress: noop,
    hasNext: false,
    position: 1,
    total: 1,
  };
  for (const item of [
    assessment,
    {
      ...assessment,
      response: {
        type: "single_select" as const,
        options: [
          { id: "a", text: "選択肢A" },
          { id: "b", text: "選択肢B" },
        ],
      },
    },
  ]) {
    const html = renderToStaticMarkup(
      createElement(AssessmentView, { ...props, assessment: item }),
    );
    assert.equal((html.match(/type="radio"/g) ?? []).length, 2);
    assert.match(html, /<label class="option"><input type="radio"/);
    assert.match(html, /type="submit" disabled="">採点する/);
    assert.doesNotMatch(html, /次の問題/);
  }
});
