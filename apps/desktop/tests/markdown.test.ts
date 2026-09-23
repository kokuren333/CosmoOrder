import test from "node:test";
import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Markdown } from "../src/components/Markdown.ts";

function html(markdown: string): string {
  return renderToStaticMarkup(createElement(Markdown, { markdown }));
}

test("GFM structures, math and code render without interpreting raw HTML", () => {
  const fence = "```";
  const markup = html(`# 見出し\n\n**bold** ~~deleted~~\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n$x^2$\n\n$$\\frac{1}{2}$$\n\n${fence}js\nconst x = 1;\n${fence}\n\n<script>alert(1)</script>`);
  assert.match(markup, /<h1>見出し<\/h1>/);
  assert.match(markup, /<strong>bold<\/strong>/);
  assert.match(markup, /<del>deleted<\/del>/);
  assert.match(markup, /<table>/);
  assert.match(markup, /katex/);
  assert.match(markup, /hljs/);
  assert.doesNotMatch(markup, /<script>/);
  assert.match(markup, /&lt;script&gt;/);
});

test("unsafe and package-relative links are not navigable", () => {
  const markup = html("[bad](javascript:alert%281%29) [local](content/a.md) [web](https://example.test)");
  assert.doesNotMatch(markup, /href="javascript:/);
  assert.doesNotMatch(markup, /href="content\/a.md"/);
  assert.match(markup, /href="https:\/\/example.test"/);
});

