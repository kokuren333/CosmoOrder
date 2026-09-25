import { ArrowDown, ArrowLeft, ArrowRight, BookOpenText, Network } from "lucide-react";
import type { LessonView, ObjectiveProgress } from "../types.ts";
import { attemptsFor, routePosition } from "../learningNavigation.ts";
import { localize, useUiLanguage } from "../i18n.ts";

export function RoutePanel({
  lesson,
  objectiveId,
  progress,
  busy,
  onSelectObjective,
  onOpenAtlas,
  onOpenResource,
  onBack,
}: {
  lesson: LessonView;
  objectiveId: string | null;
  progress: ObjectiveProgress[];
  busy: boolean;
  onSelectObjective: (id: string) => void;
  onOpenAtlas: (conceptId: string) => void;
  onOpenResource: (resourceId: string) => void;
  onBack: () => void;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const route = routePosition(lesson, objectiveId);
  const objectiveResources = route.objective === null
    ? []
    : lesson.resources.filter((resource) => resource.teaches.includes(route.objective!.id));

  return (
    <section className="route-panel" aria-labelledby="route-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">{tr("LEARNING ROUTE", "LEARNING ROUTE")}</p>
          <h1 id="route-heading">{tr("学習ルート", "Learning route")}</h1>
          <p className="lede">{lesson.manifest.title}{route.curriculum ? ` · ${route.curriculum.title}` : ""}</p>
        </div>
        <button className="ghost" onClick={onBack} disabled={busy}>
          <ArrowLeft size={16} aria-hidden="true" />{tr("Lessonへ戻る", "Back to Lesson")}
        </button>
      </div>

      <section className="route-order" aria-labelledby="route-order-heading">
        <div className="route-section-heading">
          <div>
            <p className="eyebrow">{tr("作者が定義した順序", "AUTHOR-DEFINED ORDER")}</p>
            <h2 id="route-order-heading">{tr("学習順序", "Course order")}</h2>
          </div>
          <span className="route-legend"><ArrowDown size={16} aria-hidden="true" />{tr("この順番で並んでいます", "Shown in this order")}</span>
        </div>
        {route.curriculum && route.orderedObjectives.length > 0 ? (
          <ol className="route-sequence">
            {route.orderedObjectives.map((item, index) => {
              const current = item.id === route.objective?.id;
              const concept = lesson.concepts.find((candidate) => candidate.id === item.concept);
              const attempts = attemptsFor(progress, item.id);
              return (
                <li className={current ? "route-step is-current" : "route-step"} key={item.id}>
                  <button
                    className="route-step-button"
                    aria-current={current ? "step" : undefined}
                    aria-label={tr(
                      `${index + 1}番目: ${item.description}${current ? "、現在" : ""}`,
                      `Step ${index + 1}: ${item.description}${current ? ", current" : ""}`,
                    )}
                    disabled={busy}
                    onClick={() => onSelectObjective(item.id)}
                  >
                    <span className="route-step-marker" aria-hidden="true">{current ? "●" : "○"}</span>
                    <span className="route-step-copy">
                      <span className="route-step-state">{current ? tr("現在", "Current") : tr(`順序 ${index + 1}`, `Step ${index + 1}`)}</span>
                      <strong>{item.description}</strong>
                      {concept ? <span className="meta">{concept.title}</span> : null}
                    </span>
                    <span className="route-observation">{attempts === 0 ? tr("未回答", "No attempts") : tr(`${attempts}回回答`, `${attempts} attempts`)}</span>
                  </button>
                </li>
              );
            })}
          </ol>
        ) : route.objective !== null ? (
          <div className="empty card">
            <h3>{tr("カリキュラム順は定義されていません", "No Curriculum order is defined")}</h3>
            <p>{tr("この学習目標の前後を推定せず、明示された前提だけを表示します。", "No sequence is inferred; only explicit prerequisites are shown below.")}</p>
          </div>
        ) : (
          <div className="empty card">
            <h3>{tr("学習目標がまだありません", "No learning objectives yet")}</h3>
            <p>{tr("このPackageでは学習目標の順序を表示できません。", "This Package has no objective sequence to show.")}</p>
          </div>
        )}
        <p className="meta route-disclaimer">{tr("これはPackage作者が並べた順序です。Runtimeは最適経路を推定していません。", "This order comes from the Package author. The Runtime does not infer an optimal path.")}</p>
      </section>

      {route.objective !== null ? (
        <section className="route-current card" aria-labelledby="route-current-heading">
          <p className="eyebrow">{tr("今いる場所", "CURRENT POSITION")}</p>
          <h2 id="route-current-heading">{route.objective.description}</h2>
          {route.concept ? <p className="meta">{route.concept.title}</p> : null}
          <p>{route.before
            ? tr(`カリキュラムでは「${route.before.description}」の後に置かれています。`, `In the Curriculum, this follows “${route.before.description}.”`)
            : route.curriculum
              ? tr("このカリキュラムの最初の目標です。", "This is the first objective in this Curriculum.")
              : tr("Curriculum上の位置は定義されていません。", "No Curriculum position is defined.")}</p>
          {route.after ? (
            <p className="route-next-reason"><ArrowRight size={16} aria-hidden="true" />{tr(`次は「${route.after.description}」です。作者がこの順序で定義しています。`, `Next is “${route.after.description},” following the order defined by the author.`)}</p>
          ) : route.curriculum ? (
            <p className="meta">{tr("このカリキュラムの最後の目標です。", "This is the last objective in this Curriculum.")}</p>
          ) : null}
          {objectiveResources.length > 0 ? (
            <div className="route-actions">
              {objectiveResources.map((resource) => (
                <button className="primary" key={resource.id} disabled={busy} onClick={() => onOpenResource(resource.id)}>
                  <BookOpenText size={16} aria-hidden="true" />{tr("この目標の教材を開く:", "Open learning resource:")} {resource.title}
                </button>
              ))}
            </div>
          ) : null}
          {route.concept ? (
            <button className="ghost route-atlas-link" disabled={busy} onClick={() => onOpenAtlas(route.concept!.id)}>
              <Network size={16} aria-hidden="true" />{tr("このテーマの地図を見る", "View this topic in the Atlas")}
            </button>
          ) : null}
        </section>
      ) : null}

      {route.concept !== null ? (
        <section className="route-relations" aria-labelledby="route-relations-heading">
          <div className="route-section-heading">
            <div>
              <p className="eyebrow">{tr("知識上の前提", "KNOWLEDGE PREREQUISITES")}</p>
            <h2 id="route-relations-heading">{tr("前提となるテーマ", "Topics to know first")}</h2>
            </div>
            <span className="route-legend prerequisite-legend"><Network size={16} aria-hidden="true" />{tr("点線 = 明示された前提関係", "Dashed = explicit prerequisite")}</span>
          </div>
          <div className="prerequisite-callout">
            {route.prerequisiteConcepts.length > 0 ? (
              <>
                <p>{tr(`「${route.concept.title}」には、次のテーマが前提として定義されています。`, `The Package declares these prerequisites for “${route.concept.title}.”`)}</p>
                <ul className="concept-pill-list">
                  {route.prerequisiteConcepts.map((concept) => (
                    <li key={concept.id}><button className="ghost" disabled={busy} onClick={() => onOpenAtlas(concept.id)}>{concept.title}</button></li>
                  ))}
                </ul>
              </>
            ) : (
              <p>{tr("このConceptに前提として定義されたテーマはありません。", "No explicit prerequisite Concepts are defined for this topic.")}</p>
            )}
          </div>
          {route.dependentConcepts.length > 0 ? (
            <div className="dependent-callout">
              <h3>{tr("このテーマを前提としているもの", "Concepts that require this topic")}</h3>
              <ul className="concept-pill-list">
                {route.dependentConcepts.map((concept) => (
                  <li key={concept.id}><button className="ghost" disabled={busy} onClick={() => onOpenAtlas(concept.id)}>{concept.title}</button></li>
                ))}
              </ul>
            </div>
          ) : null}
        </section>
      ) : null}
    </section>
  );
}
