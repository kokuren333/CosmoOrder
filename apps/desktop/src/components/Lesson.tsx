import { useState } from "react";
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
  const [query, setQuery] = useState("");
  const view = outline(lesson);
  const found = search(lesson, query);
  const first = readingOrder(lesson)[0];
  const progressById = new Map(
    progress.map((item) => [item.objective_id, item]),
  );
  const questionTitle = (id: string) => lesson.stimuli[id]?.text || "練習問題";
  const conceptCard = (node: ConceptNode, index: number) => (
    <article className="concept card" key={node.concept.id}>
      <div className="concept-head">
        <span className="chapter-number" aria-hidden="true">
          {String(index + 1).padStart(2, "0")}
        </span>
        <div>
          <p className="eyebrow">学ぶテーマ</p>
          <h3>{node.concept.title}</h3>
          {node.concept.requires.length > 0 ? (
            <p className="meta">
              前提となるテーマ:{" "}
              {node.concept.requires
                .map(
                  (id) =>
                    lesson.concepts.find((c) => c.id === id)?.title ??
                    "未指定のテーマ",
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
                  <p className="eyebrow">学習目標</p>
                  <h4>{objective.description}</h4>
                </div>
                <span className="badge">
                  {!observed || observed.attempts === 0
                    ? "まだ回答なし"
                    : `${observed.correct}/${observed.attempts} 正答`}
                </span>
              </div>
              <div className="learning-actions">
                <section>
                  <h5>読んで理解する</h5>
                  {resources.length === 0 ? (
                    <p className="meta">この目標の読み物はありません。</p>
                  ) : (
                    resources.map((resource) => (
                      <button
                        className="learning-link"
                        key={resource.id}
                        disabled={busy}
                        aria-label={`教材を開く:${resource.title}`}
                        onClick={() => onOpenResource(resource.id)}
                      >
                        <span>{resource.title}</span>
                        <span aria-hidden="true">→</span>
                      </button>
                    ))
                  )}
                </section>
                <section>
                  <h5>問題で確かめる</h5>
                  {assessments.length === 0 ? (
                    <p className="meta">この目標の問題はありません。</p>
                  ) : (
                    assessments.map((assessment) => (
                      <button
                        className="learning-link"
                        key={assessment.id}
                        disabled={busy}
                        aria-label={`問題を開く:${questionTitle(assessment.id)}`}
                        onClick={() => onOpenAssessment(assessment.id)}
                      >
                        <span className="question-preview">
                          {questionTitle(assessment.id)}
                        </span>
                        <span aria-hidden="true">→</span>
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
          <p className="eyebrow">LEARNING PATH</p>
          <h1 id="lesson-heading">{lesson.manifest.title}</h1>
          <p className="meta">
            {lesson.manifest.language} · バージョン {lesson.package_version}
          </p>
          <p>読んで、確かめて。ひとつずつ学びを重ねましょう。</p>
        </div>
        {first ? (
          <div>
            <button
              className="primary"
              disabled={busy}
              onClick={() => onOpenResource(first.id)}
            >
              最初の教材を読む →
            </button>
            <p className="meta">目次に沿って表示しています</p>
          </div>
        ) : null}
      </header>
      <label className="field search-field">
        <span>教材を検索</span>
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="テーマ・教材名・ID"
        />
      </label>
      {query.trim() !== "" ? (
        <div className="search-results card" aria-live="polite">
          <h2>検索結果</h2>
          <p>
            {found.resources.length +
              found.concepts.length +
              found.assessments.length}{" "}
            件
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
            <p className="eyebrow">カリキュラム</p>
            <h2>{node.curriculum.title}</h2>
            <p className="meta">{node.concepts.length} のテーマ</p>
          </div>
          {node.concepts.map(conceptCard)}
          {node.orphan_objectives.length > 0 ? (
            <p className="warning">
              テーマを特定できない学習目標が {node.orphan_objectives.length}{" "}
              件あります。
            </p>
          ) : null}
          <DeveloperDetails>
            <p>Curriculum ID: {node.curriculum.id}</p>
          </DeveloperDetails>
        </section>
      ))}
      {view.unlisted_concepts.length > 0 ? (
        <section className="curriculum">
          <h2>その他のテーマ</h2>
          {view.unlisted_concepts.map(conceptCard)}
        </section>
      ) : null}
      <DeveloperDetails label="教材の技術情報">
        <dl>
          <dt>Package ID</dt>
          <dd>{lesson.package_id}</dd>
          <dt>Schema</dt>
          <dd>{lesson.manifest.schema_version}</dd>
          <dt>Digest</dt>
          <dd>{lesson.digest}</dd>
        </dl>
      </DeveloperDetails>
    </section>
  );
}
