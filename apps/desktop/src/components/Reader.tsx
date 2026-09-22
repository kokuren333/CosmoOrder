import { Markdown } from "./Markdown.ts";
import { ArrowLeft, ArrowRight, ListTree } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import type { Resource, ResourceView } from "../types.ts";

export function Reader({
  resource,
  view,
  previous,
  next,
  onNavigate,
  onBack,
  loading,
  courseTitle,
}: {
  resource: Resource;
  view: ResourceView | null;
  previous: Resource | null;
  next: Resource | null;
  onNavigate: (id: string) => void;
  onBack: () => void;
  loading: boolean;
  courseTitle: string;
}) {
  const navigation = (label: string) => (
    <nav className="reader-nav" aria-label={label}>
      <button
        className="ghost"
        disabled={loading || previous === null}
        onClick={() => previous && onNavigate(previous.id)}
      >
        <span><ArrowLeft size={16} aria-hidden="true" />前の教材</span>
        <small>{previous?.title ?? "最初の教材です"}</small>
      </button>
      <button className="ghost" disabled={loading} onClick={onBack}>
        <ListTree size={16} aria-hidden="true" />目次へ
      </button>
      <button
        className="ghost"
        disabled={loading || next === null}
        onClick={() => next && onNavigate(next.id)}
      >
        <span>次の教材<ArrowRight size={16} aria-hidden="true" /></span>
        <small>{next?.title ?? "最後の教材です"}</small>
      </button>
    </nav>
  );
  return (
    <article className="reader" aria-labelledby="reader-heading">
      <div className="breadcrumb">
        <button className="ghost" onClick={onBack} disabled={loading}>
          {courseTitle}
        </button>
        <span aria-hidden="true">/</span>
        <span>教材を読む</span>
      </div>
      {navigation("本文の前の教材ナビゲーション")}
      <div className="reading-surface">
        <header>
          <p className="eyebrow">READ & UNDERSTAND</p>
          <h1 id="reader-heading">{resource.title}</h1>
          {resource.creator !== undefined ? (
            <p className="meta">作成: {resource.creator}</p>
          ) : null}
          {resource.attribution !== undefined ? (
            <p className="meta">{resource.attribution}</p>
          ) : null}
        </header>
        {loading ? (
          <p role="status">読み込み中…</p>
        ) : view === null ? (
          <p className="empty">本文を読み込めませんでした。</p>
        ) : (
          <Markdown content={view.content} />
        )}
        <DeveloperDetails>
          <dl>
            <dt>Resource ID</dt>
            <dd>{resource.id}</dd>
            <dt>Type</dt>
            <dd>{resource.type}</dd>
            <dt>Path</dt>
            <dd>{resource.path}</dd>
          </dl>
          {view?.content_is_untrusted ? (
            <p>
              本文はパッケージ由来のテキストです。HTMLとスクリプトは実行されません。
            </p>
          ) : null}
        </DeveloperDetails>
      </div>
      {navigation("本文の後の教材ナビゲーション")}
    </article>
  );
}
