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
          package_version: "1.0",
          schema_version: "0.1",
          digest: "private-digest",
          selected_version: "1.0",
        },
      ],
      selected: null,
      busy: false,
      onSelect: noop,
      onRefresh: noop,
    }),
  );
  assert.match(html, /<h2>教材の名前<\/h2>/);
  assert.match(
    html,
    /<details class="developer-details"><summary>技術情報<\/summary>.*private-id.*private-digest/s,
  );
  assert.doesNotMatch(html, /<details[^>]*\bopen\b/);
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
    children: "本文",
  };
  const html = renderToStaticMarkup(createElement(AppShell, props));
  assert.match(html, /aria-label="目次を表示" aria-current="page"/);
  assert.equal((html.match(/aria-current="page"/g) ?? []).length, 1);
  assert.match(html, /aria-pressed="true">文字 200%/);
  const empty = renderToStaticMarkup(
    createElement(AppShell, { ...props, section: "packages", title: null }),
  );
  assert.match(empty, /aria-label="進捗を表示" disabled=""/);
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
    stimulus: { markdown: "問題", text: "問題", content: { blocks: [] } },
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
