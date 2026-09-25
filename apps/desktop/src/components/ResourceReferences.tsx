import type { ReactElement } from "react";
import type { ResourceReference } from "../types.ts";
import { localize, useUiLanguage } from "../i18n.ts";

/** Whether the record itself may be shown to a learner. */
function isRecordVisible(source: ResourceReference): boolean {
  if (source.record_visibility) return source.record_visibility === "public";
  return source.visibility !== "private";
}

/** Whether the locator may be shown. A missing locator is never shown either. */
function isLocatorVisible(source: ResourceReference): boolean {
  if (source.locator_visibility) return source.locator_visibility === "public";
  return source.visibility === "public";
}

/** Human-readable bibliographic line, independent of any presentation markup. */
function bibliographicLine(source: ResourceReference): string | null {
  const parts = [
    source.authors?.join(", "),
    source.publisher,
    source.edition,
    source.version,
    source.updated_at ?? source.published_at,
  ].filter((part): part is string => Boolean(part));
  return parts.length > 0 ? parts.join(" · ") : null;
}

/**
 * Learner bibliography for resource-level evidence. Runtime has already removed
 * authoring provenance and hidden locators; this view is a second presentation
 * boundary and never creates a navigable link.
 */
export function ResourceReferences({ references }: { references: ResourceReference[] }): ReactElement | null {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const visible = references.filter(isRecordVisible);
  if (visible.length === 0) return null;

  return (
    <details className="resource-references">
      <summary>{tr("参考資料", "References")}</summary>
      <ul>
        {visible.map((source) => {
          const line = bibliographicLine(source);
          const locator = isLocatorVisible(source) ? source.locator : undefined;
          return (
            <li
              key={source.id}
              data-source-visibility={source.visibility}
              data-locator-visibility={locator ? "public" : "hidden"}
            >
              <h3>{source.title}</h3>
              {line ? <p>{line}</p> : null}
              {source.citation ? <p>{source.citation}</p> : null}
              {locator ? (
                <p className="source-locator">
                  <span>{tr("公開URL", "Public URL")}: </span>
                  <span dir="ltr">{locator}</span>
                </p>
              ) : null}
            </li>
          );
        })}
      </ul>
    </details>
  );
}
