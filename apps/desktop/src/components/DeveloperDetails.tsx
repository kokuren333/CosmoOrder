import type { ReactNode } from "react";
import { localize, useUiLanguage } from "../i18n.ts";

export function DeveloperDetails({
  children,
  label,
}: {
  children: ReactNode;
  label?: string;
}) {
  const language = useUiLanguage();
  const summary = label ?? localize(language, "技術情報", "Technical details");
  return (
    <details className="developer-details">
      <summary>{summary}</summary>
      <div className="details-body">{children}</div>
    </details>
  );
}
