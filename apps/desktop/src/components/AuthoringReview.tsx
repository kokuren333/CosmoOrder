import type { ReactElement } from "react";
import type { ErrorView, ReferenceRecord } from "../types.ts";
import { localize, useUiLanguage } from "../i18n.ts";

function display(value: unknown): string {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.join(", ");
  return JSON.stringify(value) ?? "";
}

/** Read-only authoring projection of schema metadata and Core/Package findings. */
export function AuthoringReview({
  references,
  diagnostics,
}: {
  references: ReferenceRecord[];
  diagnostics: ErrorView[];
}): ReactElement {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);

  return (
    <section className="authoring-review" aria-label={tr("作成者向けレビュー", "Author review")}>
      <section aria-labelledby="authoring-references-heading">
        <h2 id="authoring-references-heading">{tr("参照メタデータ", "Reference metadata")}</h2>
        {references.length === 0 ? (
          <p>{tr("Referenceはありません。", "No References are registered.")}</p>
        ) : (
          <ul>
            {references.map((reference) => (
              <li key={reference.id}>
                <h3>{reference.title ?? reference.id}</h3>
                <dl>
                  {Object.entries(reference).map(([field, value]) => (
                    <div key={field}>
                      <dt>{field}</dt>
                      <dd data-field={field}>{display(value)}</dd>
                    </div>
                  ))}
                </dl>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="authoring-diagnostics-heading">
        <h2 id="authoring-diagnostics-heading">{tr("検証・lint結果", "Validation and lint")}</h2>
        {diagnostics.length === 0 ? (
          <p>{tr("診断はありません。", "No findings.")}</p>
        ) : (
          <ol>
            {diagnostics.map((diagnostic, index) => (
              <li key={`${diagnostic.code}:${diagnostic.file}:${diagnostic.path}:${index}`} data-severity={diagnostic.severity}>
                <strong>{diagnostic.severity}</strong> <code>{diagnostic.code}</code>
                <p>{diagnostic.message}</p>
                <p>
                  {[diagnostic.file, diagnostic.path].filter(Boolean).join(" · ")}
                  {diagnostic.line === null ? "" : `:${diagnostic.line}${diagnostic.column === null ? "" : `:${diagnostic.column}`}`}
                </p>
                {diagnostic.entity_type || diagnostic.entity_id ? (
                  <p>{[diagnostic.entity_type, diagnostic.entity_id].filter(Boolean).join(": ")}</p>
                ) : null}
                {diagnostic.suggestions.length > 0 ? (
                  <ul>{diagnostic.suggestions.map((suggestion) => <li key={suggestion}>{suggestion}</li>)}</ul>
                ) : null}
              </li>
            ))}
          </ol>
        )}
      </section>
    </section>
  );
}
