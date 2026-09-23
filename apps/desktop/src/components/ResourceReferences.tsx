import type { ReactElement } from "react";
import type { ResourceSource } from "../types.ts";
import { localize, useUiLanguage } from "../i18n.ts";

/** Compact, resource-level provenance. Runtime has already removed private records. */
export function ResourceReferences({ sources }: { sources: ResourceSource[] }): ReactElement | null {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const visible = sources.filter(
    (source) => source.visibility === "public" || source.visibility === "attribution_only",
  );
  if (visible.length === 0) return null;

  return (
    <details className="resource-references">
      <summary>{tr("参考資料", "References")}</summary>
      <ul>
        {visible.map((source) => (
          <li key={source.id} data-source-visibility={source.visibility}>
            <h3>{source.title}</h3>
            {source.citation ? <p>{source.citation}</p> : null}
            {source.visibility === "public" && source.locator ? (
              <p className="source-locator">
                <span>{tr("公開URL", "Public URL")}: </span>
                <span dir="ltr">{source.locator}</span>
              </p>
            ) : null}
          </li>
        ))}
      </ul>
    </details>
  );
}
