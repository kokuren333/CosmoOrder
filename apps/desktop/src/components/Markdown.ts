import { createElement, useState, type ReactElement, type ReactNode } from "react";
import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import type { Components } from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

function safeUrl(url: string): string {
  return /^(https?:|mailto:)/i.test(url) ? defaultUrlTransform(url) : "";
}

function CodeBlock({ children, className }: { children?: ReactNode; className?: string }): ReactElement {
  const [copied, setCopied] = useState(false);
  const text = String(children ?? "").replace(/\n$/, "");
  return createElement("div", { className: "code-frame" },
    createElement("button", {
      type: "button", className: "code-copy", "aria-label": copied ? "Code copied" : "Copy code",
      onClick: () => {
        void navigator.clipboard?.writeText(text).then(() => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      },
    }, copied ? "Copied" : "Copy"),
    createElement("pre", { className: "md-pre" }, createElement("code", { className }, children)),
  );
}

const components: Components = {
  a: ({ href, children }) => {
    const safe = href ? safeUrl(href) : "";
    return createElement("a", {
      className: "md-link", href: safe || undefined, target: safe ? "_blank" : undefined,
      rel: "noreferrer noopener",
    }, children);
  },
  code: ({ className, children, node: _node, ...props }) => className
    ? createElement(CodeBlock, { className, children })
    : createElement("code", { className: "md-code", ...props }, children),
  table: ({ children }) => createElement("div", { className: "table-scroll" }, createElement("table", null, children)),
};

/** Package Markdown is parsed into React elements. Raw HTML is ignored. */
export function Markdown({ markdown }: { markdown: string }): ReactElement {
  return createElement("div", { className: "markdown" }, createElement(ReactMarkdown, {
    remarkPlugins: [remarkGfm, remarkMath],
    rehypePlugins: [[rehypeKatex, { throwOnError: false, trust: false, maxSize: 20, maxExpand: 1000 }], rehypeHighlight],
    components,
    urlTransform: safeUrl,
    children: markdown,
  }));
}
