// Read-only views over a lesson. This module only groups what the package
// already declares; it never decides what is correct, what is next, or what a
// learner has mastered. Ordering comes from the package's own arrays.

import type { Assessment, Concept, Curriculum, LessonView, Objective, Resource } from "./types.ts";

export interface ConceptNode {
  concept: Concept;
  objectives: Objective[];
  resources: Resource[];
  assessments: Assessment[];
}

export interface CurriculumNode {
  curriculum: Curriculum;
  concepts: ConceptNode[];
  /** Objectives named by the curriculum but declared by no listed concept. */
  orphan_objectives: Objective[];
}

export interface LessonOutline {
  curricula: CurriculumNode[];
  /** Concepts no curriculum lists, so nothing is silently unreachable. */
  unlisted_concepts: ConceptNode[];
}

function conceptNode(
  lesson: LessonView,
  concept: Concept,
  objectives: Objective[],
): ConceptNode {
  const objectiveIds = new Set(objectives.map((objective) => objective.id));
  return {
    concept,
    objectives,
    resources: lesson.resources.filter((resource) =>
      resource.teaches.some((id) => objectiveIds.has(id)),
    ),
    assessments: lesson.assessments.filter((assessment) =>
      assessment.measures.some((id) => objectiveIds.has(id)),
    ),
  };
}

/** Group a lesson into curricula, concepts, objectives, resources and items. */
export function outline(lesson: LessonView): LessonOutline {
  const objectivesByConcept = new Map<string, Objective[]>();
  for (const objective of lesson.objectives) {
    const list = objectivesByConcept.get(objective.concept) ?? [];
    list.push(objective);
    objectivesByConcept.set(objective.concept, list);
  }
  const byId = new Map(lesson.concepts.map((concept) => [concept.id, concept]));
  const listed = new Set<string>();
  const curricula: CurriculumNode[] = lesson.curricula.map((curriculum) => {
    const concepts: ConceptNode[] = [];
    const orphanObjectives: Objective[] = [];
    for (const objectiveId of curriculum.objectives) {
      const objective = lesson.objectives.find((item) => item.id === objectiveId);
      if (objective === undefined) {
        // A validated package cannot reference a missing objective, so this is
        // only reachable when the renderer is handed inconsistent data.
        continue;
      }
      const concept = byId.get(objective.concept);
      if (concept === undefined) {
        orphanObjectives.push(objective);
        continue;
      }
      listed.add(concept.id);
      const existing = concepts.find((node) => node.concept.id === concept.id);
      if (existing === undefined) {
        concepts.push(conceptNode(lesson, concept, [objective]));
      } else {
        existing.objectives.push(objective);
      }
    }
    return { curriculum, concepts, orphan_objectives: orphanObjectives };
  });
  const unlisted = lesson.concepts
    .filter((concept) => !listed.has(concept.id))
    .map((concept) => conceptNode(lesson, concept, objectivesByConcept.get(concept.id) ?? []));
  return { curricula, unlisted_concepts: unlisted };
}

/** Resources that teach an objective, in package order. */
export function resourcesForObjective(lesson: LessonView, objectiveId: string): Resource[] {
  return lesson.resources.filter((resource) => resource.teaches.includes(objectiveId));
}

/** Items that measure an objective, in package order. */
export function assessmentsForObjective(
  lesson: LessonView,
  objectiveId: string,
): Assessment[] {
  return lesson.assessments.filter((assessment) => assessment.measures.includes(objectiveId));
}

/** A flat reading order for keyboard navigation and previous/next links. */
export function readingOrder(lesson: LessonView): Resource[] {
  const seen = new Set<string>();
  const ordered: Resource[] = [];
  for (const curriculum of lesson.curricula) {
    for (const objectiveId of curriculum.objectives) {
      for (const resource of resourcesForObjective(lesson, objectiveId)) {
        if (!seen.has(resource.id)) {
          seen.add(resource.id);
          ordered.push(resource);
        }
      }
    }
  }
  for (const resource of lesson.resources) {
    if (!seen.has(resource.id)) {
      seen.add(resource.id);
      ordered.push(resource);
    }
  }
  return ordered;
}

/** Case-insensitive search over the entities a reader can open. */
export function search(
  lesson: LessonView,
  query: string,
): { resources: Resource[]; concepts: Concept[]; assessments: Assessment[] } {
  const needle = query.trim().toLowerCase();
  if (needle === "") {
    return { resources: [], concepts: [], assessments: [] };
  }
  const matches = (value: string) => value.toLowerCase().includes(needle);
  return {
    resources: lesson.resources.filter(
      (resource) => matches(resource.id) || matches(resource.title),
    ),
    concepts: lesson.concepts.filter(
      (concept) => matches(concept.id) || matches(concept.title),
    ),
    assessments: lesson.assessments.filter((assessment) => matches(assessment.id)),
  };
}
