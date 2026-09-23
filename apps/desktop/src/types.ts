// Types mirroring the desktop shell's DTOs. Field names stay exactly as the
// Rust structs serialize them, so nothing is renamed or reinterpreted here.

/** Inline content IR produced by `osmium_core::content`. */
export type Span =
  | { type: "text"; text: string }
  | { type: "code"; text: string }
  | { type: "emphasis"; spans: Span[] }
  | { type: "strong"; spans: Span[] }
  | { type: "strikethrough"; spans: Span[] }
  | { type: "math"; tex: string; display: boolean }
  | { type: "link"; url: string; href: string; spans: Span[] };

export type Block =
  | { type: "heading"; level: number; spans: Span[] }
  | { type: "paragraph"; spans: Span[] }
  | { type: "list"; ordered: boolean; tight: boolean; items: ListItem[] }
  | { type: "block_quote"; blocks: Block[] }
  | { type: "code"; language: string | null; text: string }
  | { type: "math"; tex: string }
  | { type: "table"; headers: Span[][]; rows: Span[][][] }
  | { type: "html"; text: string }
  | { type: "rule" };

export interface ListItem {
  blocks: Block[];
}

export interface Content {
  blocks: Block[];
}

/** Core-compiled renderer-neutral content crossing the Desktop IPC boundary. */
export interface ContentView {
  /** Plain-text preview derived from the IR. */
  text: string;
  content: Content;
}

/** One diagnostic as Core or Package reported it. */
export interface ErrorView {
  code: string;
  message: string;
  file: string | null;
  path: string;
  suggestions: string[];
  entity_type?: string;
  entity_id?: string;
}

export interface CommandError {
  diagnostics: ErrorView[];
}

export interface StatusView {
  home: string;
  database: string;
  packages: number;
  state_version: number;
}

export interface PackageView {
  package_id: string;
  package_version: string;
  schema_version: string;
  title: string;
  entity_counts: Record<string, number>;
  digest: string;
  selected_version: string;
}

export interface Manifest {
  schema_version: string;
  package_id: string;
  package_version: string;
  title: string;
  language: string;
  capabilities: { required: string[]; optional: string[] };
  entities: Record<string, string>;
  extensions: Record<string, unknown>;
}

export interface Concept {
  id: string;
  title: string;
  requires: string[];
  description?: string;
}

export interface Objective {
  id: string;
  concept: string;
  description: string;
}

export interface Curriculum {
  id: string;
  title: string;
  objectives: string[];
}

export interface Resource {
  id: string;
  type: string;
  title: string;
  path: string;
  teaches: string[];
  creator?: string;
  license?: string;
  attribution?: string;
}

export interface AssessmentOption {
  id: string;
  text: string;
}

export type AssessmentResponse =
  | { type: "single_select"; options: AssessmentOption[] }
  | { type: "boolean" };

export interface Assessment {
  id: string;
  revision: string;
  measures: string[];
  stimulus: { markdown: string };
  response: AssessmentResponse;
  evaluation: { type: string; answer: string | boolean };
  feedback: { markdown: string };
}

export interface LessonView {
  package_id: string;
  package_version: string;
  digest: string;
  manifest: Manifest;
  concepts: Concept[];
  objectives: Objective[];
  curricula: Curriculum[];
  resources: Resource[];
  assessments: Assessment[];
  /** Compiled stimulus Markdown, keyed by assessment ID. */
  stimuli: Record<string, ContentView>;
}

export interface ResourceView {
  package_id: string;
  package_version: string;
  digest: string;
  resource: Resource;
  content: Content;
  content_is_untrusted: boolean;
}

export interface ObjectiveProgress {
  objective_id: string;
  attempts: number;
  correct: number;
  accuracy: number | null;
  last_score: number | null;
  last_timestamp: string | null;
}

export interface LearningEvent {
  event_id: string;
  event_type: string;
  package_id: string;
  package_version: string;
  assessment_id: string;
  assessment_revision: string;
  objective_ids: string[];
  concept_ids: string[];
  response: string | boolean;
  score: number;
  correct: boolean;
  timestamp: string;
  duration_ms: number | null;
  hints_used: number | null;
  evaluator: { id: string; version: string };
}

export interface AttemptView {
  event: LearningEvent;
  replayed: boolean;
  correct: boolean;
  score: number;
  feedback: { markdown: string };
  feedback_content: ContentView;
  evaluator: { id: string; version: string };
  objective_ids: string[];
  assessment_id: string;
}
