import { useState } from "react";
import { ArrowRight, BookOpenText, ChevronRight, ClipboardCheck, Search } from "lucide-react";
import { outline, search, readingOrder, assessmentsForObjective } from "../outline.ts";
import type { ConceptNode } from "../outline.ts";
import type { LessonView, ObjectiveProgress } from "../types.ts";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

export function Lesson({ lesson, progress, onOpenResource, onOpenAssessment, onOpenAtlas, onOpenObjective, currentObjectiveId, onOpenRoute, busy }: {
  lesson: LessonView;
  progress: ObjectiveProgress[];
  busy: boolean;
  currentObjectiveId: string | null;
  onOpenResource: (id: string) => void;
  onOpenAssessment: (id: string) => void;
  onOpenAtlas: (conceptId: string) => void;
  onOpenObjective: (objectiveId: string) => void;
  onOpenRoute: () => void;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const [query, setQuery] = useState("");
  const [openSections, setOpenSections] = useState<Set<string>>(() => new Set(lesson.curricula[0] ? [lesson.curricula[0].id] : ["__unlisted__"]));
  const view = outline(lesson);
  const found = search(lesson, query);
  const first = readingOrder(lesson)[0];
  const progressById = new Map(progress.map((item) => [item.objective_id, item]));
  const orderedIds = [...new Set(lesson.curricula.flatMap((curriculum) => curriculum.objectives))];
  const currentId = orderedIds.includes(currentObjectiveId ?? "") ? currentObjectiveId! : orderedIds[0] ?? null;
  const currentIndex = currentId === null ? -1 : orderedIds.indexOf(currentId);
  const objectiveAt = (index: number) => index < 0 ? null : lesson.objectives.find((item) => item.id === orderedIds[index]) ?? null;
  const before = objectiveAt(currentIndex - 1);
  const current = currentId === null ? null : lesson.objectives.find((item) => item.id === currentId) ?? null;
  const next = objectiveAt(currentIndex + 1);
  const unlistedObjectiveIds = new Set(view.unlisted_concepts.flatMap((node) => node.objectives.map((objective) => objective.id)));
  const unlistedResources = lesson.resources.filter((resource) => resource.teaches.some((id) => unlistedObjectiveIds.has(id)));
  const questionTitle = (id: string) => lesson.stimuli[id]?.text || tr("練習問題", "Practice question");
  const conceptCard = (node: ConceptNode, index: number) => (
    <article className="concept card" key={node.concept.id}>
      <div className="concept-head"><span className="chapter-number" aria-hidden="true">{String(index + 1).padStart(2, "0")}</span><div>
        <p className="eyebrow">{tr("テーマ", "TOPIC")}</p><h3>{node.concept.title}</h3>
      {node.concept.requires.length > 0 ? <div className="lesson-prerequisites"><span className="meta">{tr("前提を見る:", "Prerequisites:")}</span>{node.concept.requires.map((id) => {
          const concept = lesson.concepts.find((item) => item.id === id);
          return concept ? <button type="button" className="concept-prerequisite-link" key={id} disabled={busy} onClick={() => onOpenAtlas(id)}>{concept.title}</button> : null;
        })}</div> : null}
        {lesson.package_id === "org.example/cardiovascular-atlas-validation" && node.concept.id === "heart-failure" ? <aside className="relation-prototype"><span className="eyebrow">{tr("関係の表示案 · 検証用", "RELATED TOPIC · PROTOTYPE")}</span><p>{tr("心不全症候群はRAASと関連します（前提関係ではありません）。", "Heart failure syndrome is related to RAAS (not a prerequisite).")}</p><button type="button" className="ghost" disabled={busy} onClick={() => onOpenAtlas("raas")}>{tr("RAASを見る", "Explore RAAS")}</button></aside> : null}
      </div></div>
      <ul className="objective-list">{node.objectives.map((objective) => {
        const assessments = assessmentsForObjective(lesson, objective.id);
        const observed = progressById.get(objective.id);
        return <li key={objective.id}><div className="objective"><div><p className="eyebrow">{tr("学習目標", "LEARNING OBJECTIVE")}</p><h4>{objective.description}</h4></div>
          {assessments.length > 0 ? <span className="badge">{observed?.attempts ? `${observed.attempts} ${tr("回の回答", "answers recorded")}` : tr("回答記録なし", "No answers recorded")}</span> : null}</div>
          <div className="learning-actions"><section><h5><ClipboardCheck size={16} aria-hidden="true" />{tr("問題 · 確かめる", "PRACTICE · Check")}</h5>
            {assessments.length === 0 ? <p className="meta">{tr("関連する問題はありません。", "No linked questions.")}</p> : assessments.map((assessment) => <button className="learning-link" key={assessment.id} disabled={busy} aria-label={tr(`問題を開く:${questionTitle(assessment.id)}`, `Open assessment: ${questionTitle(assessment.id)}`)} onClick={() => onOpenAssessment(assessment.id)}><span className="question-preview">{questionTitle(assessment.id)}</span><ChevronRight size={17} aria-hidden="true" /></button>)}
          </section></div>
          <DeveloperDetails><dl><dt>Objective ID</dt><dd>{objective.id}</dd></dl></DeveloperDetails>
        </li>;
      })}</ul>
      <DeveloperDetails><dl><dt>Concept ID</dt><dd>{node.concept.id}</dd><dt>Requires</dt><dd>{node.concept.requires.join(", ") || "—"}</dd></dl></DeveloperDetails>
    </article>
  );
  return <section aria-labelledby="lesson-heading">
    <header className="lesson-hero"><div><p className="eyebrow">{tr("学習内容", "YOUR COURSE")}</p><h1 id="lesson-heading">{lesson.manifest.title}</h1>
      <p className="meta">{lesson.manifest.language.toLowerCase().startsWith("ja") ? tr("日本語", "Japanese") : lesson.manifest.language} · {tr("バージョン", "Version")} {lesson.package_version}</p>
      <p className="meta">{lesson.concepts.length} {tr("テーマ", "topics")} · {lesson.curricula.length} {tr("セクション", "sections")}</p>
    </div><div className="lesson-primary-actions">{first ? <div><button className="primary" disabled={busy} onClick={() => onOpenResource(first.id)}>{tr("最初の教材を読む", "Read first resource")} <ArrowRight size={17} aria-hidden="true" /></button><p className="meta">{tr("カリキュラム順", "Curriculum order")}</p></div> : null}
      {lesson.objectives.length > 0 ? <button className="ghost" disabled={busy} onClick={onOpenRoute} aria-label={tr("学習ルートを見る", "View learning route")}>{tr("ルート全体を見る", "View full route")} <ArrowRight size={17} aria-hidden="true" /></button> : null}</div></header>
    {current && currentIndex >= 0 ? <nav className="lesson-current-route" aria-label={tr("カリキュラムの現在位置", "Current curriculum position")}><p className="eyebrow">{tr("カリキュラム順", "CURRICULUM ORDER")}</p><div className="lesson-current-route-items">
      <div><span className="meta">{tr("前", "BEFORE")}</span><strong>{before?.description ?? tr("先頭", "Start")}</strong></div><div className="current"><span className="meta">{tr("今", "NOW")}</span><strong>{current.description}</strong></div>
      <div><span className="meta">{tr("次", "NEXT")}</span>{next ? <button type="button" disabled={busy} onClick={() => onOpenObjective(next.id)}>{next.description}</button> : <strong>{tr("最後", "End")}</strong>}</div>
    </div>{before ? <button type="button" className="ghost" disabled={busy} onClick={() => onOpenObjective(before.id)}>{tr("前の目標へ", "Open previous objective")}</button> : null}</nav> : null}
    <label className="field search-field"><span><Search size={15} aria-hidden="true" />{tr("教材を検索", "Search course")}</span><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder={tr("テーマ・教材名・識別子", "Topic, resource, or ID")} /></label>
    {query.trim() !== "" ? <div className="search-results card" aria-live="polite"><h2>{tr("検索結果", "Search results")}</h2><p>{found.resources.length + found.concepts.length + found.assessments.length} {tr("件", "results")}</p><ul>
      {found.concepts.map((item) => <li key={`c-${item.id}`}><button disabled={busy} onClick={() => onOpenAtlas(item.id)}>{item.title}</button></li>)}
      {found.resources.map((item) => <li key={`r-${item.id}`}><button disabled={busy} onClick={() => onOpenResource(item.id)}>{item.title}</button></li>)}
      {found.assessments.map((item) => <li key={`a-${item.id}`}><button disabled={busy} onClick={() => onOpenAssessment(item.id)}>{questionTitle(item.id)}</button></li>)}
    </ul></div> : null}
    {view.curricula.map((node) => {
      const objectiveIds = new Set(node.curriculum.objectives);
      const resources = lesson.resources.filter((resource) => resource.teaches.some((id) => objectiveIds.has(id)));
      const assessed = node.curriculum.objectives.filter((id) => lesson.assessments.some((assessment) => assessment.measures.includes(id)));
      const observed = assessed.filter((id) => (progressById.get(id)?.attempts ?? 0) > 0).length;
      const answers = assessed.reduce((sum, id) => sum + (progressById.get(id)?.attempts ?? 0), 0);
      const isOpen = openSections.has(node.curriculum.id);
      return <details className="curriculum curriculum-section" key={node.curriculum.id} open={isOpen} onToggle={(event) => { const isNowOpen = event.currentTarget.open; setOpenSections((sections) => { const nextState = new Set(sections); if (isNowOpen) nextState.add(node.curriculum.id); else nextState.delete(node.curriculum.id); return nextState; }); }}><summary className="section-heading curriculum-summary"><span><p className="eyebrow">{tr("セクション", "SECTION")}</p><h2>{node.curriculum.title}</h2></span><span className="meta">{observed}/{assessed.length} {tr("評価対象に回答", "assessed objectives answered")} · {answers} {tr("回", "answers")}</span></summary>
        {isOpen ? <>
          {resources.length > 0 ? <section className="curriculum-resources" aria-label={tr("このセクションの教材", "Section resources")}><h3><BookOpenText size={16} aria-hidden="true" />{tr("このセクションの教材", "Section resources")}</h3><ul>{resources.map((resource) => <li key={resource.id}><button className="learning-link" disabled={busy} aria-label={tr(`教材を開く:${resource.title}`, `Open resource: ${resource.title}`)} onClick={() => onOpenResource(resource.id)}><span>{resource.title}</span><ChevronRight size={17} aria-hidden="true" /></button><details className="resource-support-details"><summary>{tr("対応する学習目標", "Objectives supported")}</summary><ul>{resource.teaches.filter((id) => objectiveIds.has(id)).map((id) => <li key={id}>{lesson.objectives.find((item) => item.id === id)?.description ?? id}</li>)}</ul></details></li>)}</ul></section> : null}
          {node.concepts.map(conceptCard)}{node.orphan_objectives.length > 0 ? <p className="warning">{tr("テーマ未指定の学習目標:", "Objectives without a topic:")} {node.orphan_objectives.length}</p> : null}<DeveloperDetails><p>Curriculum ID: {node.curriculum.id}</p></DeveloperDetails>
        </> : null}
      </details>;
    })}
    {view.unlisted_concepts.length > 0 ? <details className="curriculum curriculum-section" open={openSections.has("__unlisted__")} onToggle={(event) => { const isNowOpen = event.currentTarget.open; setOpenSections((sections) => { const nextState = new Set(sections); if (isNowOpen) nextState.add("__unlisted__"); else nextState.delete("__unlisted__"); return nextState; }); }}><summary className="section-heading curriculum-summary"><span><p className="eyebrow">{tr("その他", "OTHER")}</p><h2>{tr("カリキュラム外のテーマ", "Other topics")}</h2></span><span className="meta">{view.unlisted_concepts.length} {tr("テーマ", "topics")}</span></summary>{openSections.has("__unlisted__") ? <>{unlistedResources.length > 0 ? <section className="curriculum-resources" aria-label={tr("このセクションの教材", "Section resources")}><h3><BookOpenText size={16} aria-hidden="true" />{tr("このセクションの教材", "Section resources")}</h3><ul>{unlistedResources.map((resource) => <li key={resource.id}><button className="learning-link" disabled={busy} aria-label={tr(`教材を開く:${resource.title}`, `Open resource: ${resource.title}`)} onClick={() => onOpenResource(resource.id)}><span>{resource.title}</span><ChevronRight size={17} aria-hidden="true" /></button></li>)}</ul></section> : null}{view.unlisted_concepts.map(conceptCard)}</> : null}</details> : null}
    <DeveloperDetails label={tr("教材の技術情報", "Package details")}><dl><dt>Package ID</dt><dd>{lesson.package_id}</dd><dt>Schema</dt><dd>{lesson.manifest.schema_version}</dd><dt>Digest</dt><dd>{lesson.digest}</dd></dl></DeveloperDetails>
  </section>;
}
