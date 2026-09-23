import { Markdown } from "./Markdown.ts";
import { ArrowLeft, ArrowRight, ListTree } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { ResourceReferences } from "./ResourceReferences.tsx";
import type { Resource, ResourceView } from "../types.ts";
import { localize, useUiLanguage } from "../i18n.ts";

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
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const navigation = (label: string) => (
    <nav className="reader-nav" aria-label={label}>
      <button
        className="ghost"
        disabled={loading || previous === null}
        onClick={() => previous && onNavigate(previous.id)}
      >
        <span><ArrowLeft size={16} aria-hidden="true" />{tr("前の教材", "Previous resource")}</span>
        <small>{previous?.title ?? tr("最初の教材です", "First resource")}</small>
      </button>
      <button className="ghost" disabled={loading} onClick={onBack}>
        <ListTree size={16} aria-hidden="true" />{tr("目次へ", "Back to contents")}
      </button>
      <button
        className="ghost"
        disabled={loading || next === null}
        onClick={() => next && onNavigate(next.id)}
      >
        <span>{tr("次の教材", "Next resource")}<ArrowRight size={16} aria-hidden="true" /></span>
        <small>{next?.title ?? tr("最後の教材です", "Last resource")}</small>
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
        <span>{tr("教材", "Resource")}</span>
      </div>
      {navigation("本文の前の教材ナビゲーション")}
      <div className="reading-surface">
        <header>
          <p className="eyebrow">{tr("読んで理解する", "READ & UNDERSTAND")}</p>
          <h1 id="reader-heading">{resource.title}</h1>
          {resource.creator !== undefined ? (
          <p className="meta">{tr("作成:", "Created by:")} {resource.creator}</p>
          ) : null}
          {resource.attribution !== undefined ? (
            <p className="meta">{resource.attribution}</p>
          ) : null}
        </header>
        {loading ? (
          <p role="status">{tr("読み込み中…", "Loading…")}</p>
        ) : view === null ? (
          <p className="empty">{tr("本文を読み込めませんでした。", "Could not load this resource.")}</p>
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
              {tr("Package本文のHTMLやスクリプトは実行されません。", "Package HTML and scripts are not executed.")}
            </p>
          ) : null}
        </DeveloperDetails>
      </div>
      <ResourceReferences sources={view?.sources ?? []} />
      {navigation("本文の後の教材ナビゲーション")}
    </article>
  );
}
