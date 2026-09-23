import test from "node:test";
import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Markdown } from "../src/components/Markdown.ts";
import type { Content } from "../src/types.ts";

function html(content: Content): string {
  return renderToStaticMarkup(createElement(Markdown, { content }));
}

test("renders typed Markdown IR, GFM structures, TeX and inert highlighted code", () => {
  const content: Content = { blocks: [
    { type: "heading", level: 1, spans: [{ type: "text", text: "見出し" }] },
    { type: "paragraph", spans: [
      { type: "strong", spans: [{ type: "text", text: "bold" }] },
      { type: "strikethrough", spans: [{ type: "text", text: "old" }] },
      { type: "math", tex: "x^2", display: false },
    ] },
    { type: "table", headers: [[{ type: "text", text: "A" }]], rows: [[[ { type: "text", text: "東京" } ]]] },
    { type: "math", tex: "\\frac{1}{2}", },
    { type: "code", language: "js", text: "const x = 1;\n" },
    { type: "code", language: "html", text: "</code><img src=x onerror=alert(1)>" },
    { type: "html", text: "<script>alert(1)</script>" },
  ] };
  const markup = html(content);
  assert.match(markup, /<h1>見出し<\/h1>/);
  assert.match(markup, /<strong>bold<\/strong>/);
  assert.match(markup, /<del>old<\/del>/);
  assert.match(markup, /<table>/);
  assert.match(markup, /katex/);
  assert.match(markup, /hljs/);
  assert.doesNotMatch(markup, /<script>/);
  assert.doesNotMatch(markup, /<img /);
  assert.match(markup, /&lt;script&gt;/);
});

test("only Core-classified absolute links can navigate", () => {
  const content: Content = { blocks: [{ type: "paragraph", spans: [
    { type: "link", url: "", href: "content/a.md", spans: [{ type: "text", text: "local" }] },
    { type: "link", url: "https://example.test", href: "https://example.test", spans: [{ type: "text", text: "web" }] },
  ] }] };
  const markup = html(content);
  assert.doesNotMatch(markup, /href="content\/a.md"/);
  assert.match(markup, /href="https:\/\/example.test"/);
});
