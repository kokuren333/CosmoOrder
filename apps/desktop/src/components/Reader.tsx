import type { ReactElement } from "react";
import { Markdown } from "./Markdown.ts";
import type { Resource, ResourceView } from "../types.ts";

/** The Markdown reader. The body is the compiled content IR, never raw HTML. */
export function Reader({
  resource,
  view,
  previous,
  next,
  onNavigate,
  onBack,
  loading,
}: {
  resource: Resource;
  view: ResourceView | null;
  previous: Resource | null;
  next: Resource | null;
  onNavigate: (resourceId: string) => void;
  onBack: () => void;
  loading: boolean;
}): ReactElement {
  return (
    <article className="panel reader" aria-labelledby="reader-heading">
      <div className="panel-head">
        <h2 id="reader-heading">{resource.title}</h2>
        <div className="row">
          <button type="button" onClick={onBack}>
            目次へ
          </button>
          <button
            type="button"
            onClick={() => previous !== null && onNavigate(previous.id)}
            disabled={previous === null}
          >
            前へ
          </button>
          <button
            type="button"
            onClick={() => next !== null && onNavigate(next.id)}
            disabled={next === null}
          >
            次へ
          </button>
        </div>
      </div>
      <p className="meta">
        resource <code>{resource.id}</code> · type {resource.type} · path{" "}
        <code>{resource.path}</code>
      </p>
      {resource.creator !== undefined ? (
        <p className="meta">作成: {resource.creator}</p>
      ) : null}
      {resource.attribution !== undefined ? (
        <p className="meta">{resource.attribution}</p>
      ) : null}
      {loading ? (
        <p aria-live="polite">読み込み中…</p>
      ) : view === null ? (
        <p className="empty">本文を読み込めませんでした。</p>
      ) : (
        <>
          <Markdown content={view.content} />
          {view.content_is_untrusted ? (
            <p className="meta">
              本文はパッケージ由来のテキストです。HTMLとスクリプトは実行されません。
            </p>
          ) : null}
        </>
      )}
    </article>
  );
}
