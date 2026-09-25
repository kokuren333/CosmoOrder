import type { Curriculum, LessonView, Objective, ObjectiveProgress } from "./types.ts";

export interface RoutePosition {
  objective: Objective | null;
  concept: LessonView["concepts"][number] | null;
  curriculum: Curriculum | null;
  orderedObjectives: Objective[];
  index: number;
  before: Objective | null;
  after: Objective | null;
  prerequisiteConcepts: LessonView["concepts"];
  dependentConcepts: LessonView["concepts"];
}

/** First objective according to an explicit Curriculum, with document-order fallback. */
export function defaultObjectiveId(lesson: LessonView): string | null {
  for (const curriculum of lesson.curricula) {
    const objective = curriculum.objectives.find((id) =>
      lesson.objectives.some((item) => item.id === id),
    );
    if (objective !== undefined) return objective;
  }
  return lesson.objectives[0]?.id ?? null;
}

/** Find a study objective without inventing order when no Curriculum lists it. */
export function objectiveForConcept(lesson: LessonView, conceptId: string): string | null {
  for (const curriculum of lesson.curricula) {
    const objectiveId = curriculum.objectives.find((id) =>
      lesson.objectives.some((item) => item.id === id && item.concept === conceptId),
    );
    if (objectiveId !== undefined) return objectiveId;
  }
  return lesson.objectives.find((item) => item.concept === conceptId)?.id ?? null;
}

/** Resolve only explicit author data for Route. No recommendation is inferred. */
export function routePosition(lesson: LessonView, objectiveId: string | null): RoutePosition {
  const objective = objectiveId === null
    ? null
    : lesson.objectives.find((item) => item.id === objectiveId) ?? null;
  const concept = objective === null
    ? null
    : lesson.concepts.find((item) => item.id === objective.concept) ?? null;
  const curriculum = objective === null
    ? null
    : lesson.curricula.find((item) => item.objectives.includes(objective.id)) ?? null;
  const orderedObjectives = curriculum === null
    ? []
    : curriculum.objectives.flatMap((id) => {
        const item = lesson.objectives.find((candidate) => candidate.id === id);
        return item === undefined ? [] : [item];
      });
  const index = objective === null ? -1 : orderedObjectives.findIndex((item) => item.id === objective.id);
  const conceptById = new Map(lesson.concepts.map((item) => [item.id, item]));
  const prerequisiteConcepts = concept?.requires.flatMap((id) => {
    const item = conceptById.get(id);
    return item === undefined ? [] : [item];
  }) ?? [];
  const dependentConcepts = concept === null
    ? []
    : lesson.concepts.filter((item) => item.requires.includes(concept.id));
  return {
    objective,
    concept,
    curriculum,
    orderedObjectives,
    index,
    before: index > 0 ? orderedObjectives[index - 1] ?? null : null,
    after: index >= 0 ? orderedObjectives[index + 1] ?? null : null,
    prerequisiteConcepts,
    dependentConcepts,
  };
}

/** Learning state is shown as observed answer counts, never inferred knowledge. */
export function attemptsFor(progress: ObjectiveProgress[], objectiveId: string): number {
  return progress.find((item) => item.objective_id === objectiveId)?.attempts ?? 0;
}
