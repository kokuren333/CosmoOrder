import type { ReactElement } from "react";
import { useState } from "react";
import { outline, search } from "../outline.ts";
import type { LessonView, ObjectiveProgress } from "../types.ts";

/** Curriculum / concept / objective navigation for one package. */
export function Lesson({
  lesson,
  progress,
  onOpenResource,
  onOpenAssessment,
  onShowProgress,
  onShowHistory,
}: {
  lesson: LessonView;
  progress: ObjectiveProgress[];
  onOpenResource: (resourceId: string) => void;
  onOpenAssessment: (assessmentId: string) => void;
  onShowProgress: () => void;
  onShowHistory: () => void;
}): ReactElement {
  const [query, setQuery] = useState("");
  const view = outline(lesson);
  const found = search(lesson, query);
  const progressById = new Map(progress.map((item) => [item.objective_id, item]));

  const conceptSections = [
    ...view.curricula.flatMap((curriculum) => curriculum.concepts),
    ...view.unlisted_concepts,
  ];

  return (
    <section className="panel" aria-labelledby="lesson-heading">
      <div className="panel-head">
        <h2 id="lesson-heading">{lesson.manifest.title}</h2>
        <div className="row">
          <button type="button" aria-label="進捗を表示" onClick={onShowProgress}>
            進捗
          </button>
          <button type="button" aria-label="履歴を表示" onClick={onShowHistory}>
            履歴
          </button>
        </div>
      </div>
      <p className="meta">
        {lesson.package_id} · version {lesson.package_version} · language{" "}
        {lesson.manifest.language} · digest {lesson.digest.slice(0, 16)}…
      </p>

      <label className="field">
        <span>教材を検索</span>
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="concept / resource / assessment ID"
        />
      </label>
      {query.trim() !== "" ? (
        <div className="search-results" aria-live="polite">
          <p>
            {found.resources.length + found.concepts.length + found.assessments.length} 件
          </p>
          <ul>
            {found.concepts.map((concept) => (
              <li key={`c-${concept.id}`}>
                Concept: {concept.title} <code>{concept.id}</code>
              </li>
            ))}
            {found.resources.map((resource) => (
              <li key={`r-${resource.id}`}>
                <button type="button" onClick={() => onOpenResource(resource.id)}>
                  Resource: {resource.title}
                </button>
              </li>
            ))}
            {found.assessments.map((assessment) => (
              <li key={`a-${assessment.id}`}>
                <button type="button" onClick={() => onOpenAssessment(assessment.id)}>
                  Assessment: {assessment.id}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {view.curricula.map((node) => (
        <article className="curriculum" key={node.curriculum.id}>
          <h3>{node.curriculum.title}</h3>
          <p className="meta">
            curriculum <code>{node.curriculum.id}</code> · {node.concepts.length} concept
          </p>
          {node.orphan_objectives.length > 0 ? (
            <p className="warning">
              Concept を特定できない objective: {node.orphan_objectives.length} 件
            </p>
          ) : null}
        </article>
      ))}

      {conceptSections.map((node) => (
        <article className="concept" key={node.concept.id}>
          <h3>{node.concept.title}</h3>
          <p className="meta">
            concept <code>{node.concept.id}</code>
            {node.concept.requires.length > 0
              ? ` · requires ${node.concept.requires.join(", ")}`
              : ""}
          </p>
          <ul className="objective-list">
            {node.objectives.map((objective) => {
              const item = progressById.get(objective.id);
              return (
                <li key={objective.id}>
                  <div className="objective">
                    <span className="objective-text">{objective.description}</span>
                    <span className="objective-id">
                      <code>{objective.id}</code>
                    </span>
                    <span className="objective-progress">
                      {item === undefined || item.attempts === 0
                        ? "未挑戦"
                        : `${item.correct}/${item.attempts} 正答`}
                    </span>
                  </div>
                  <div className="row">
                    {node.resources
                      .filter((resource) => resource.teaches.includes(objective.id))
                      .map((resource) => (
                        <button
                          type="button"
                          key={resource.id}
                          aria-label={`教材を開く:${resource.title}`}
                          onClick={() => onOpenResource(resource.id)}
                        >
                          読む: {resource.title}
                        </button>
                      ))}
                    {node.assessments
                      .filter((assessment) => assessment.measures.includes(objective.id))
                      .map((assessment) => (
                        <button
                          type="button"
                          key={assessment.id}
                          aria-label={`問題を開く:${assessment.id}`}
                          onClick={() => onOpenAssessment(assessment.id)}
                        >
                          解く: {assessment.id}
                        </button>
                      ))}
                  </div>
                </li>
              );
            })}
          </ul>
        </article>
      ))}
    </section>
  );
}
