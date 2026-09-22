// Renders the content IR produced by Core. Every node becomes a React element,
// so there is no HTML sink: package text can never introduce markup, and a link
// target is only ever used after Core classified it.
//
// This module uses `createElement` instead of JSX so the same file is directly
// executable by the test runner as well as by the bundler.

import { createElement, type ReactElement, type ReactNode } from "react";
import type { Block, Content, Span } from "../types.ts";

const HEADINGS = ["h1", "h2", "h3", "h4", "h5", "h6"] as const;

function inline(spans: Span[], keyPrefix: string): ReactNode[] {
  return spans.map((span, index) => {
    const key = `${keyPrefix}.${index}`;
    switch (span.type) {
      case "text":
        return span.text;
      case "code":
        return createElement("code", { className: "md-code", key }, span.text);
      case "emphasis":
        return createElement("em", { key }, inline(span.spans, key));
      case "strong":
        return createElement("strong", { key }, inline(span.spans, key));
      case "link":
        // An in-package target is inert: the reader navigates with the package
        // outline, never by following a package-supplied path.
        return createElement(
          "a",
          {
            className: "md-link",
            href: span.url === "" ? undefined : span.url,
            key,
            rel: "noreferrer noopener",
            target: span.url === "" ? undefined : "_blank",
            title: span.url === "" ? `in-package: ${span.href}` : span.url,
          },
          inline(span.spans, key),
        );
      default:
        return null;
    }
  });
}

function renderBlocks(items: Block[], keyPrefix: string): ReactNode[] {
  return items.map((block, index) => {
    const key = `${keyPrefix}.${index}`;
    switch (block.type) {
      case "heading": {
        const level = Math.min(Math.max(block.level, 1), 6);
        const tag = HEADINGS[level - 1] ?? "h6";
        return createElement(tag, { key }, inline(block.spans, key));
      }
      case "paragraph":
        return createElement("p", { key }, inline(block.spans, key));
      case "list": {
        const listItems = block.items.map((item, itemIndex) =>
          createElement(
            "li",
            { key: `${key}.${itemIndex}` },
            renderBlocks(item.blocks, `${key}.${itemIndex}`),
          ),
        );
        return createElement(block.ordered ? "ol" : "ul", { key }, listItems);
      }
      case "block_quote":
        return createElement("blockquote", { key }, renderBlocks(block.blocks, key));
      case "code":
        return createElement(
          "pre",
          { className: "md-pre", key },
          createElement(
            "code",
            { "data-language": block.language ?? undefined },
            block.text,
          ),
        );
      case "html":
        // Block HTML is inert: it is displayed as literal source text.
        return createElement(
          "pre",
          { className: "md-html", key, "aria-label": "unrendered HTML source" },
          createElement("code", null, block.text),
        );
      case "rule":
        return createElement("hr", { key });
      default:
        return null;
    }
  });
}

/** Render compiled Markdown content. */
export function Markdown({ content }: { content: Content }): ReactElement {
  return createElement("div", { className: "markdown" }, renderBlocks(content.blocks, "b"));
}
