// The renderer must turn package text into elements only. These tests read the
// component output as markup, so a regression that reintroduced an HTML sink
// would show up as an unexpected element or attribute.

import test from "node:test";
import assert from "node:assert/strict";
import { renderToStaticMarkup } from "react-dom/server";
import { Markdown } from "../src/components/Markdown.ts";
import type { Content } from "../src/types.ts";

function html(content: Content): string {
  return renderToStaticMarkup(Markdown({ content }));
}

test("raw HTML in the content IR is displayed as text, not markup", () => {
  const content: Content = {
    blocks: [
      { type: "html", text: "<script>alert('x')</script>" },
      {
        type: "paragraph",
        spans: [{ type: "text", text: "<img src=x onerror=alert(1)>" }],
      },
    ],
  };
  const markup = html(content);
  assert.ok(!markup.includes("<script"), markup);
  assert.ok(!markup.includes("<img"), markup);
  assert.ok(markup.includes("&lt;script&gt;"), markup);
  assert.ok(markup.includes("&lt;img"), markup);
});

test("a rejected link becomes text and never an anchor", () => {
  // Core emits a rejected destination as emphasis over the label text.
  const content: Content = {
    blocks: [
      {
        type: "paragraph",
        spans: [{ type: "emphasis", spans: [{ type: "text", text: "click" }] }],
      },
    ],
  };
  const markup = html(content);
  assert.ok(!markup.includes("<a "), markup);
  assert.ok(markup.includes("click"), markup);
});

test("an in-package link stays inert and an absolute link is marked safe", () => {
  const content: Content = {
    blocks: [
      {
        type: "paragraph",
        spans: [
          {
            type: "link",
            url: "",
            href: "content/other.md",
            spans: [{ type: "text", text: "inside" }],
          },
          {
            type: "link",
            url: "https://example.test/a",
            href: "https://example.test/a",
            spans: [{ type: "text", text: "web" }],
          },
        ],
      },
    ],
  };
  const markup = html(content);
  assert.ok(markup.includes('title="in-package: content/other.md"'), markup);
  assert.ok(!markup.includes('href="content/other.md"'), markup);
  assert.ok(markup.includes('href="https://example.test/a"'), markup);
  assert.ok(markup.includes('rel="noreferrer noopener"'), markup);
});

test("headings, lists, code and quotes render as elements", () => {
  const content: Content = {
    blocks: [
      { type: "heading", level: 2, spans: [{ type: "text", text: "見出し" }] },
      {
        type: "list",
        ordered: true,
        tight: true,
        items: [
          { blocks: [{ type: "paragraph", spans: [{ type: "text", text: "one" }] }] },
          { blocks: [{ type: "paragraph", spans: [{ type: "text", text: "two" }] }] },
        ],
      },
      { type: "code", language: "rust", text: "fn main() {}" },
      {
        type: "block_quote",
        blocks: [{ type: "paragraph", spans: [{ type: "text", text: "quoted" }] }],
      },
      { type: "rule" },
    ],
  };
  const markup = html(content);
  assert.ok(markup.includes("<h2>見出し</h2>"), markup);
  assert.ok(markup.includes("<ol>"), markup);
  assert.ok(markup.includes("<li>"), markup);
  assert.ok(markup.includes('data-language="rust"'), markup);
  assert.ok(markup.includes("<blockquote>"), markup);
  assert.ok(markup.includes("<hr/>") || markup.includes("<hr>"), markup);
});
