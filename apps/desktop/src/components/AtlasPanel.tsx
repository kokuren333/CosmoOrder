import { useEffect, useMemo, useState } from "react";
import { ArrowLeft, BookOpenText, Map as MapIcon, Search, Target } from "lucide-react";
import type { ConceptSearchView, LessonView, ObjectiveProgress, PackageContextView } from "../types.ts";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

const MAX_VISIBLE_RELATIONS = 6;

function strings(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

export function AtlasPanel({
  lesson,
  selectedConceptId,
  context,
  progress,
  busy,
  onSearch,
  onRecenter,
  onStudyConcept,
  onOpenResource,
  onOpenAssessment,
  onBackToRoute,
  onBackToLesson,
}: {
  lesson: LessonView;
  selectedConceptId: string;
  context: PackageContextView | null;
  progress: ObjectiveProgress[];
  busy: boolean;
  onSearch: (query: string) => Promise<ConceptSearchView>;
  onRecenter: (conceptId: string) => void;
  onStudyConcept: (conceptId: string) => void;
  onOpenResource: (resourceId: string) => void;
  onOpenAssessment: (assessmentId: string) => void;
  onBackToRoute: () => void;
  onBackToLesson: () => void;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ConceptSearchView | null>(null);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    const value = query.trim();
    if (value === "") {
      setSearchResults(null);
      setSearching(false);
      return;
    }
    let active = true;
    setSearching(true);
    const timer = setTimeout(() => {
      void onSearch(value).then((result) => {
        if (active) setSearchResults(result);
      }).catch(() => {
        if (active) setSearchResults(null);
      }).finally(() => {
        if (active) setSearching(false);
      });
    }, 180);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [onSearch, query]);

  const nodes = context?.nodes ?? [];
  const relations = context?.relations ?? [];
  const target = nodes.find((node) => node.kind === "concept" && node.id === selectedConceptId);
  const conceptById = useMemo(
    () => new Map(nodes.filter((node) => node.kind === "concept").map((node) => [node.id, node])),
    [nodes],
  );
  const prerequisiteIds = [...new Set(relations
    .filter((relation) => relation.relation === "requires" && relation.from_id === selectedConceptId && !relation.incoming)
    .map((relation) => relation.to_id))];
  const dependentIds = [...new Set(relations
    .filter((relation) => relation.relation === "requires" && relation.from_id === selectedConceptId && relation.incoming)
    .map((relation) => relation.to_id))];
  const prerequisites = prerequisiteIds.flatMap((id) => conceptById.has(id) ? [conceptById.get(id)!] : []);
  const dependents = dependentIds.flatMap((id) => conceptById.has(id) ? [conceptById.get(id)!] : []);
  const visiblePrerequisites = prerequisites.slice(0, MAX_VISIBLE_RELATIONS);
  const visibleDependents = dependents.slice(0, MAX_VISIBLE_RELATIONS);
  const objectiveNodes = nodes.filter((node) => node.kind === "objective" && node.entity.concept === selectedConceptId);
  const objectiveIds = new Set(objectiveNodes.map((node) => node.id));
  const resourceNodes = nodes.filter((node) => node.kind === "resource" && strings(node.entity.teaches).some((id) => objectiveIds.has(id)));
  const assessmentNodes = nodes.filter((node) => node.kind === "assessment" && strings(node.entity.measures).some((id) => objectiveIds.has(id)));
  const curricula = nodes.filter((node) => node.kind === "curriculum" && strings(node.entity.objectives).some((id) => objectiveIds.has(id)));
  const contextConceptCount = nodes.filter((node) => node.kind === "concept").length;
  const observedByObjective = new Map(progress.map((item) => [item.objective_id, item.attempts]));
  const conceptAttempts = objectiveNodes.reduce((sum, node) => sum + (observedByObjective.get(node.id) ?? 0), 0);

  const conceptButton = (node: (typeof nodes)[number], relationLabel: string) => (
    <li key={`${node.kind}:${node.id}`}>
      <button className="atlas-neighbor" disabled={busy} onClick={() => onRecenter(node.id)}>
        <span>{node.title}</span>
        <small>{relationLabel}</small>
      </button>
    </li>
  );

  return (
    <section className="atlas-panel" aria-labelledby="atlas-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">{tr("KNOWLEDGE ATLAS", "KNOWLEDGE ATLAS")}</p>
          <h1 id="atlas-heading">{tr("知識の地図", "Knowledge Atlas")}</h1>
          <p className="lede">{lesson.manifest.title}</p>
        </div>
        <div className="atlas-header-actions">
          <button className="ghost" onClick={onBackToRoute} disabled={busy}>
            <ArrowLeft size={16} aria-hidden="true" />{tr("ルートへ戻る", "Back to Route")}
          </button>
          <button className="ghost" onClick={onBackToLesson} disabled={busy}>
            <BookOpenText size={16} aria-hidden="true" />{tr("Lessonへ戻る", "Back to Lesson")}
          </button>
        </div>
      </div>

      <label className="field atlas-search">
        <span><Search size={15} aria-hidden="true" />{tr("この教材のテーマを検索", "Search topics in this course")}</span>
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={tr("テーマ名を入力", "Type a topic name")}
          aria-controls="atlas-search-results"
        />
      </label>
      {query.trim() !== "" ? (
        <div className="atlas-search-results card" id="atlas-search-results" aria-live="polite">
          {searching ? <p role="status">{tr("検索中…", "Searching…")}</p> : searchResults === null ? (
            <p>{tr("検索結果を読み込めませんでした。", "Could not load search results.")}</p>
          ) : searchResults.results.length === 0 ? (
            <p>{tr("一致するテーマはありません。", "No matching topics.")}</p>
          ) : (
            <>
              <p>{tr(`${searchResults.total}件の候補`, `${searchResults.total} results`)}{searchResults.truncated ? tr("（先頭20件を表示）", " (showing the first 20)") : ""}</p>
              <ul>
                {searchResults.results.map((result) => (
                  <li key={result.id}>
                    <button className="learning-link" disabled={busy} onClick={() => { onRecenter(result.id); setQuery(""); }}>
                      <span>{result.title}</span><MapIcon size={16} aria-hidden="true" />
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      ) : null}

      {context?.truncated ? (
        <p className="notice" role="status">{tr("近くの関係のみ表示しています。さらに絞り込むには検索してください。", "Only the local neighborhood is shown. Search to recenter elsewhere in the Package.")}</p>
      ) : null}
      {context === null && busy ? (
        <p className="empty" role="status">{tr("近くのテーマを読み込んでいます…", "Loading the local neighborhood…")}</p>
      ) : target === undefined ? (
        <div className="empty card">
          <h2>{tr("このテーマの情報を読み込めませんでした", "Could not load this topic")}</h2>
          <button className="ghost" onClick={onBackToLesson}>{tr("Lessonへ戻る", "Back to Lesson")}</button>
        </div>
      ) : (
        <>
          <section className="atlas-neighborhood" aria-labelledby="atlas-neighborhood-heading">
            <div className="route-section-heading">
              <div>
                <p className="eyebrow">{tr("このテーマの周辺", "LOCAL NEIGHBORHOOD")}</p>
                <h2 id="atlas-neighborhood-heading">{tr("選択中のテーマと前提", "Selected topic and prerequisites")}</h2>
              </div>
              <p className="meta">{tr(`${contextConceptCount}件のConceptを近傍から表示`, `${contextConceptCount} Concepts in the local view`)}</p>
            </div>
            {prerequisites.length + dependents.length === 0 ? (
              <div className="empty card">
                <h3>{tr("Conceptの前提関係はまだ定義されていません", "No Concept prerequisites are defined here")}</h3>
                <p>{tr("このテーマの周辺に表示できる前提関係はありません。教材と学習目標は下に表示します。", "There are no declared prerequisite links in this neighborhood. Related objectives and resources appear below.")}</p>
              </div>
            ) : (
              <div className="atlas-relation-grid">
                <section className="atlas-relation-lane atlas-prerequisite-lane" aria-labelledby="atlas-prerequisites-heading">
                  <h3 id="atlas-prerequisites-heading">{tr("先に必要なテーマ", "Topics needed first")}</h3>
                  <p className="meta">{tr("点線の関係", "Dashed relation")}</p>
                  {visiblePrerequisites.length > 0 ? (
                    <ul>{visiblePrerequisites.map((node) => conceptButton(node, tr("このテーマに必要", "Required here")))}</ul>
                  ) : <p className="meta">{tr("明示された前提はありません", "No explicit prerequisites")}</p>}
                  {prerequisites.length > visiblePrerequisites.length ? <p className="meta">{tr(`他${prerequisites.length - visiblePrerequisites.length}件は近傍表示の上限外です`, `${prerequisites.length - visiblePrerequisites.length} more outside the visible cap`)}</p> : null}
                </section>
                <section className="atlas-focus" aria-label={tr("選択中のテーマ", "Selected topic")}>
                  <p className="eyebrow">{tr("選択中", "SELECTED")}</p>
                  <h3>{target.title}</h3>
                  <button className="primary" disabled={busy} onClick={() => onStudyConcept(selectedConceptId)}>
                    <Target size={16} aria-hidden="true" />{tr("このテーマを学ぶ", "Study this topic")}
                  </button>
                </section>
                <section className="atlas-relation-lane atlas-dependent-lane" aria-labelledby="atlas-dependents-heading">
                  <h3 id="atlas-dependents-heading">{tr("このテーマの次につながる", "Topics that build on this")}</h3>
                  <p className="meta">{tr("点線の関係", "Dashed relation")}</p>
                  {visibleDependents.length > 0 ? (
                    <ul>{visibleDependents.map((node) => conceptButton(node, tr("ここを前提とする", "Requires this topic")))}</ul>
                  ) : <p className="meta">{tr("このテーマを前提とするConceptはありません", "No Concepts require this topic")}</p>}
                  {dependents.length > visibleDependents.length ? <p className="meta">{tr(`他${dependents.length - visibleDependents.length}件は近傍表示の上限外です`, `${dependents.length - visibleDependents.length} more outside the visible cap`)}</p> : null}
                </section>
              </div>
            )}
            <p className="meta atlas-legend"><span className="legend-dash" aria-hidden="true" />{tr("点線は教材に定義された前提関係です。学習順序とは別です。", "Dashed links are declared prerequisites in this course, separate from the learning order.")}</p>
          </section>

          <section className="atlas-details" aria-labelledby="atlas-details-heading">
            <div className="route-section-heading">
              <div>
                <p className="eyebrow">{tr("学習に戻る", "RETURN TO LEARNING")}</p>
                <h2 id="atlas-details-heading">{tr("このテーマに関係するもの", "Related learning material")}</h2>
              </div>
            </div>
            <div className="atlas-detail-grid">
              <section className="card" aria-labelledby="atlas-objectives-heading">
                <h3 id="atlas-objectives-heading">{tr("学習目標", "Learning objectives")}</h3>
                {objectiveNodes.length === 0 ? <p className="meta">{tr("関連する学習目標はありません。", "No linked objectives.")}</p> : (
                  <ul className="atlas-detail-list">
                    {objectiveNodes.map((node) => (
                      <li key={node.id}>
                        <strong>{node.title}</strong>
                        <span className="meta">{tr(`回答記録: ${observedByObjective.get(node.id) ?? 0}回`, `Answer records: ${observedByObjective.get(node.id) ?? 0} attempts`)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
              <section className="card" aria-labelledby="atlas-resources-heading">
                <h3 id="atlas-resources-heading">{tr("教材", "Resources")}</h3>
                {resourceNodes.length === 0 ? <p className="meta">{tr("この近傍に教材はありません。", "No resources in this neighborhood.")}</p> : (
                  <ul className="atlas-detail-list">
                    {resourceNodes.map((node) => (
                      <li key={node.id}><button className="learning-link" disabled={busy} onClick={() => onOpenResource(node.id)}><span>{node.title}</span><BookOpenText size={16} aria-hidden="true" /></button></li>
                    ))}
                  </ul>
                )}
              </section>
              <section className="card" aria-labelledby="atlas-assessments-heading">
                <h3 id="atlas-assessments-heading">{tr("問題", "Assessments")}</h3>
                {assessmentNodes.length === 0 ? <p className="meta">{tr("この近傍に問題はありません。", "No assessments in this neighborhood.")}</p> : (
                  <ul className="atlas-detail-list">
                    {assessmentNodes.map((node) => (
                      <li key={node.id}><button className="learning-link" disabled={busy} onClick={() => onOpenAssessment(node.id)}><span>{node.title}</span><Target size={16} aria-hidden="true" /></button></li>
                    ))}
                  </ul>
                )}
              </section>
              <section className="card" aria-labelledby="atlas-curricula-heading">
                <h3 id="atlas-curricula-heading">{tr("カリキュラム上の位置", "Curriculum position")}</h3>
                {curricula.length === 0 ? <p className="meta">{tr("この近傍にCurriculum順の定義はありません。", "No Curriculum ordering is declared in this neighborhood.")}</p> : (
                  <ul className="atlas-detail-list">
                    {curricula.map((node) => {
                      const order = strings(node.entity.objectives);
                      const positions = [...objectiveIds].flatMap((id) => {
                        const index = order.indexOf(id);
                        return index < 0 ? [] : [`${index + 1}/${order.length}`];
                      });
                      return <li key={node.id}><strong>{node.title}</strong><span className="meta">{positions.join(", ") || tr("順序あり", "ordered")}</span></li>;
                    })}
                  </ul>
                )}
              </section>
            </div>
          </section>
          <section className="atlas-observations" aria-labelledby="atlas-observations-heading">
            <h2 id="atlas-observations-heading">{tr("あなたの回答記録", "Your answer records")}</h2>
            <p className="meta">{tr(`このテーマの学習目標に対する回答は合計${conceptAttempts}回です。教材で定義された関係とは別の記録です。`, `${conceptAttempts} answers are recorded across this topic’s objectives. These records are separate from relationships defined by the course.`)}</p>
          </section>
          <DeveloperDetails label={tr("テーマの技術情報", "Topic details")}>
            <dl><dt>Package ID</dt><dd>{lesson.package_id}</dd><dt>Concept ID</dt><dd>{selectedConceptId}</dd><dt>Context bounds</dt><dd>{context?.depth ?? 0} hops · {context?.nodes.length ?? 0} nodes{context?.truncated ? " · truncated" : ""}</dd></dl>
          </DeveloperDetails>
        </>
      )}
    </section>
  );
}
