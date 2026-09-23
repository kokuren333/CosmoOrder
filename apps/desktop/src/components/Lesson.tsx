import { useState } from "react";
import {
  ArrowRight,
  BookOpenText,
  ChevronRight,
  ClipboardCheck,
  Search,
} from "lucide-react";
import {
  outline,
  search,
  readingOrder,
  resourcesForObjective,
  assessmentsForObjective,
} from "../outline.ts";
import type { ConceptNode } from "../outline.ts";
import type { LessonView, ObjectiveProgress } from "../types.ts";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

export function Lesson({
  lesson,
  progress,
  onOpenResource,
  onOpenAssessment,
  busy,
}: {
  lesson: LessonView;
  progress: ObjectiveProgress[];
  busy: boolean;
  onOpenResource: (id: string) => void;
  onOpenAssessment: (id: string) => void;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const [query, setQuery] = useState("");
  const view = outline(lesson);
  const found = search(lesson, query);
  const first = readingOrder(lesson)[0];
  const progressById = new Map(
    progress.map((item) => [item.objective_id, item]),
  );
  const questionTitle = (id: string) => lesson.stimuli[id]?.text || tr("練習問題", "Practice question");
  const conceptCard = (node: ConceptNode, index: number) => (
    <article className="concept card" key={node.concept.id}>
      <div className="concept-head">
        <span className="chapter-number" aria-hidden="true">
          {String(index + 1).padStart(2, "0")}
        </span>
        <div>
          <p className="eyebrow">CONCEPT · {tr("学ぶテーマ", "Topic")}</p>
          <h3>{node.concept.title}</h3>
          {node.concept.requires.length > 0 ? (
            <p className="meta">
              {tr("前提となるConcepts:", "Prerequisite concepts:")}{" "}
              {node.concept.requires
                .map(
                  (id) =>
                    lesson.concepts.find((c) => c.id === id)?.title ??
                    tr("未指定のテーマ", "Unspecified concept"),
                )
                .join("、")}
            </p>
          ) : null}
        </div>
      </div>
      <ul className="objective-list">
        {node.objectives.map((objective) => {
          const observed = progressById.get(objective.id);
          const resources = resourcesForObjective(lesson, objective.id);
          const assessments = assessmentsForObjective(lesson, objective.id);
          return (
            <li key={objective.id}>
              <div className="objective">
                <div>
                  <p className="eyebrow">OBJECTIVE · {tr("学習目標", "Learning objective")}</p>
                  <h4>{objective.description}</h4>
                </div>
                <span className="badge">
                  {!observed || observed.attempts === 0
                    ? tr("まだ回答なし", "No attempts")
                    : `${observed.correct}/${observed.attempts} ${tr("正答", "correct")}`}
                </span>
              </div>
              <div className="learning-actions">
                <section>
                  <h5><BookOpenText size={16} aria-hidden="true" />RESOURCE · {tr("読む", "Read")}</h5>
                  {resources.length === 0 ? (
                    <p className="meta">{tr("関連するResourceはありません。", "No related resources.")}</p>
                  ) : (
                    resources.map((resource) => (
                      <button
                        className="learning-link"
                        key={resource.id}
                        disabled={busy}
                        aria-label={tr(`教材を開く:${resource.title}`, `Open resource: ${resource.title}`)}
                        onClick={() => onOpenResource(resource.id)}
                      >
                        <span>{resource.title}</span>
                        <ChevronRight size={17} aria-hidden="true" />
                      </button>
                    ))
                  )}
                </section>
                <section>
                  <h5><ClipboardCheck size={16} aria-hidden="true" />ASSESSMENT · {tr("確かめる", "Check")}</h5>
                  {assessments.length === 0 ? (
                    <p className="meta">{tr("関連するAssessmentはありません。", "No related assessments.")}</p>
                  ) : (
                    assessments.map((assessment) => (
                      <button
                        className="learning-link"
                        key={assessment.id}
                        disabled={busy}
                        aria-label={tr(`問題を開く:${questionTitle(assessment.id)}`, `Open assessment: ${questionTitle(assessment.id)}`)}
                        onClick={() => onOpenAssessment(assessment.id)}
                      >
                        <span className="question-preview">
                          {questionTitle(assessment.id)}
                        </span>
                        <ChevronRight size={17} aria-hidden="true" />
                      </button>
                    ))
                  )}
                </section>
              </div>
              <DeveloperDetails>
                <dl>
                  <dt>Objective ID</dt>
                  <dd>{objective.id}</dd>
                </dl>
              </DeveloperDetails>
            </li>
          );
        })}
      </ul>
      <DeveloperDetails>
        <dl>
          <dt>Concept ID</dt>
          <dd>{node.concept.id}</dd>
          <dt>Requires</dt>
          <dd>{node.concept.requires.join(", ") || "—"}</dd>
        </dl>
      </DeveloperDetails>
    </article>
  );
  return (
    <section aria-labelledby="lesson-heading">
      <header className="lesson-hero">
        <div>
          <p className="eyebrow">PACKAGE OVERVIEW</p>
          <h1 id="lesson-heading">{lesson.manifest.title}</h1>
          <p className="meta">{lesson.manifest.language} · {tr("バージョン", "Version")} {lesson.package_version}</p>
          <p className="meta">{lesson.concepts.length} Concepts · {lesson.objectives.length} Objectives · {lesson.resources.length} Resources · {lesson.assessments.length} Assessments</p>
          <p>{tr("Concept → Objective → Resource / Assessmentの関係", "Concept → Objective → Resource / Assessment relationships")}</p>
        </div>
        {first ? (
          <div>
            <button
              className="primary"
              disabled={busy}
              onClick={() => onOpenResource(first.id)}
            >
              {tr("最初の教材を読む", "Read first resource")} <ArrowRight size={17} aria-hidden="true" />
            </button>
            <p className="meta">{tr("カリキュラム順に表示", "Ordered by curriculum")}</p>
          </div>
        ) : null}
      </header>
      <label className="field search-field">
        <span><Search size={15} aria-hidden="true" />{tr("教材を検索", "Search package")}</span>
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={tr("テーマ・教材名・ID", "Concept, title, or ID")}
        />
      </label>
      {query.trim() !== "" ? (
        <div className="search-results card" aria-live="polite">
          <h2>{tr("検索結果", "Search results")}</h2>
          <p>
            {found.resources.length +
              found.concepts.length +
              found.assessments.length}{" "}
            {tr("件", "results")}
          </p>
          <ul>
            {found.concepts.map((concept) => (
              <li key={`c-${concept.id}`}>{concept.title}</li>
            ))}
            {found.resources.map((resource) => (
              <li key={`r-${resource.id}`}>
                <button
                  disabled={busy}
                  onClick={() => onOpenResource(resource.id)}
                >
                  {resource.title}
                </button>
              </li>
            ))}
            {found.assessments.map((assessment) => (
              <li key={`a-${assessment.id}`}>
                <button
                  disabled={busy}
                  onClick={() => onOpenAssessment(assessment.id)}
                >
                  {questionTitle(assessment.id)}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {view.curricula.map((node) => (
        <section className="curriculum" key={node.curriculum.id}>
          <div className="section-heading">
            <p className="eyebrow">{tr("カリキュラム", "Curriculum")}</p>
            <h2>{node.curriculum.title}</h2>
            <p className="meta">{node.concepts.length} {tr("Concepts", "concepts")}</p>
          </div>
          {node.concepts.map(conceptCard)}
          {node.orphan_objectives.length > 0 ? (
            <p className="warning">
              {tr("Concept未指定のObjective:", "Objectives without a concept:")} {node.orphan_objectives.length}
            </p>
          ) : null}
          <DeveloperDetails>
            <p>Curriculum ID: {node.curriculum.id}</p>
          </DeveloperDetails>
        </section>
      ))}
      {view.unlisted_concepts.length > 0 ? (
        <section className="curriculum">
          <h2>{tr("その他のConcepts", "Other concepts")}</h2>
          {view.unlisted_concepts.map(conceptCard)}
        </section>
      ) : null}
      <DeveloperDetails label={tr("教材の技術情報", "Package details")}>
        <dl>
          <dt>Package ID</dt>
          <dd>{lesson.package_id}</dd>
          <dt>Schema</dt>
          <dd>{lesson.manifest.schema_version}</dd>
          <dt>Digest</dt>
          <dd>{lesson.digest}</dd>
          {lesson.resources.some((resource) => resource.creator) ? <><dt>Resource creators</dt><dd>{[...new Set(lesson.resources.map((resource) => resource.creator).filter(Boolean))].join(", ")}</dd></> : null}
          {lesson.resources.some((resource) => resource.license) ? <><dt>Resource licenses</dt><dd>{[...new Set(lesson.resources.map((resource) => resource.license).filter(Boolean))].join(", ")}</dd></> : null}
        </dl>
      </DeveloperDetails>
    </section>
  );
}
