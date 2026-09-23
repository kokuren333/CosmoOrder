import { createElement, useState, type ReactElement, type ReactNode } from "react";
import parse from "html-react-parser";
import katex from "katex";
import { common, createLowlight } from "lowlight";
import type { Block, Content, Span } from "../types.ts";
import { useUiLanguage } from "../i18n.ts";

const lowlight = createLowlight(common);
const headingTags = ["h1", "h2", "h3", "h4", "h5", "h6"] as const;

function highlightNodes(nodes: Array<{ type: string; value?: string; tagName?: string; properties?: Record<string, unknown>; children?: unknown[] }>, prefix: string): ReactNode[] {
  return nodes.map((node, index) => {
    const key = `${prefix}.${index}`;
    if (node.type === "text") return node.value ?? "";
    if (node.type !== "element") return null;
    const props: Record<string, unknown> = { key };
    const classes = node.properties?.className;
    if (Array.isArray(classes)) props.className = classes.join(" ");
    return createElement(node.tagName ?? "span", props, highlightNodes((node.children ?? []) as typeof nodes, key));
  });
}

function CodeBlock({ language, text }: { language: string | null; text: string }): ReactElement {
  const uiLanguage = useUiLanguage();
  const [copied, setCopied] = useState(false);
  let code: ReactNode = text;
  if (language) {
    try {
      code = highlightNodes(lowlight.highlight(language, text).children as never, "hl");
    } catch {
      // Unknown fence languages remain plain, inert text.
    }
  }
  return createElement("div", { className: "code-frame" },
    createElement("button", {
      type: "button", className: "code-copy",
      "aria-label": copied ? (uiLanguage === "ja" ? "コードをコピーしました" : "Code copied") : (uiLanguage === "ja" ? "コードをコピー" : "Copy code"),
      onClick: () => {
        void navigator.clipboard?.writeText(text).then(() => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      },
    }, copied ? (uiLanguage === "ja" ? "コピー済み" : "Copied") : (uiLanguage === "ja" ? "コピー" : "Copy")),
    createElement("pre", { className: "md-pre" }, createElement("code", { className: language ? `language-${language}` : undefined }, code)),
  );
}

function inline(spans: Span[], prefix: string): ReactNode[] {
  return spans.map((span, index) => {
    const key = `${prefix}.${index}`;
    switch (span.type) {
      case "text": return span.text;
      case "code": return createElement("code", { className: "md-code", key }, span.text);
      case "emphasis": return createElement("em", { key }, inline(span.spans, key));
      case "strong": return createElement("strong", { key }, inline(span.spans, key));
      case "strikethrough": return createElement("del", { key }, inline(span.spans, key));
      case "math": return createElement("span", { key, className: "math-inline" }, parse(katex.renderToString(span.tex, { output: "htmlAndMathml", throwOnError: false, trust: false, maxSize: 20, maxExpand: 1000 })));
      case "link": return createElement("a", {
        className: "md-link", href: span.url || undefined,
        title: span.url ? undefined : `in-package: ${span.href}`,
        target: span.url ? "_blank" : undefined, rel: "noreferrer noopener", key,
      }, inline(span.spans, key));
      default: return null;
    }
  });
}

function blocks(blocksValue: Block[], prefix: string): ReactNode[] {
  return blocksValue.map((block, index) => {
    const key = `${prefix}.${index}`;
    switch (block.type) {
      case "heading": return createElement(headingTags[Math.min(Math.max(block.level, 1), 6) - 1] ?? "h6", { key }, inline(block.spans, key));
      case "paragraph": return createElement("p", { key }, inline(block.spans, key));
      case "list": return createElement(block.ordered ? "ol" : "ul", { key }, block.items.map((item, i) => createElement("li", { key: `${key}.${i}` }, blocks(item.blocks, `${key}.${i}`))));
      case "block_quote": return createElement("blockquote", { key }, blocks(block.blocks, key));
      case "code": return createElement(CodeBlock, { key, language: block.language, text: block.text });
      case "math": return createElement("div", { className: "katex-display", key }, parse(katex.renderToString(block.tex, { displayMode: true, output: "htmlAndMathml", throwOnError: false, trust: false, maxSize: 20, maxExpand: 1000 })));
      case "table": return createElement("div", { className: "table-scroll", key }, createElement("table", null,
        createElement("thead", null, createElement("tr", null, block.headers.map((cell, i) => createElement("th", { key: i, scope: "col" }, inline(cell, `${key}.h${i}`))))),
        createElement("tbody", null, block.rows.map((row, r) => createElement("tr", { key: r }, row.map((cell, c) => createElement("td", { key: c }, inline(cell, `${key}.${r}.${c}`))))))));
      case "html": return createElement("pre", { className: "md-html", key }, createElement("code", null, block.text));
      case "rule": return createElement("hr", { key });
      default: return null;
    }
  });
}

/** Render only the typed, inert Core content IR. */
export function Markdown({ content }: { content: Content }): ReactElement {
  return createElement("div", { className: "markdown" }, blocks(content.blocks, "b"));
}
