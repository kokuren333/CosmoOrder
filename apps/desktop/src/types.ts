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
  severity: string;
  message: string;
  file: string | null;
  line: number | null;
  column: number | null;
  path: string;
  suggestions: string[];
  entity_type?: string;
  entity_id?: string;
}

/** Source-manifest Reference metadata for an authoring or review surface. */
export interface ReferenceRecord {
  id: string;
  kind: "url" | "doi" | "isbn" | "citation" | "local_file" | "package_asset" | "manual";
  visibility: "public" | "attribution_only" | "private";
  title?: string;
  locator?: string;
  citation?: string;
  record_visibility?: "public" | "private";
  locator_visibility?: "public" | "hidden";
  type?: "webpage" | "article" | "book" | "guideline" | "dataset" | "document" | "other";
  publisher?: string;
  authors?: string[];
  published_at?: string;
  updated_at?: string;
  accessed_at?: string;
  edition?: string;
  version?: string;
  identifiers?: { doi?: string; isbn?: string };
  content_hash?: string;
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
  integrity_error?: string | null;
}

export interface UninstallReport {
  package_id: string;
  package_version: string;
  digest: string;
}

export type PackageEntityKind = "concept" | "objective" | "curriculum" | "resource" | "assessment";

export interface PackageContextNode {
  id: string;
  kind: PackageEntityKind;
  title: string;
  depth: number;
  entity: Record<string, unknown>;
}

export interface PackageContextRelation {
  relation: string;
  from_kind: PackageEntityKind;
  from_id: string;
  to_kind: PackageEntityKind;
  to_id: string;
  incoming: boolean;
}

/** Core's bounded, read-only Package-local neighborhood for Route/Atlas views. */
export interface PackageContextView {
  target: { id: string; kind: PackageEntityKind; title: string; detail: Record<string, unknown> };
  depth: number;
  nodes: PackageContextNode[];
  relations: PackageContextRelation[];
  truncated: boolean;
  prerequisite_depth: number | null;
  content_is_untrusted: boolean;
}

/** Small Concept-title search result set returned by the Core read model. */
export interface ConceptSearchView {
  query: string;
  total: number;
  limit: number;
  results: Array<{
    id: string;
    kind: "concept";
    title: string;
    detail: Record<string, unknown>;
  }>;
  truncated: boolean;
}

export interface BuildReport {
  package_id: string;
  package_version: string;
  digest: string;
  files: number;
  output: string;
  archive_sha256: string | null;
}

/** Explicit installed package version selected in the Library. */
export interface InstalledPackageRef {
  package_id: string;
  package_version: string;
}

/** Source-only validation/lint response; never supplied to learner views. */
export interface SourceReviewView {
  source_directory: string;
  valid: boolean;
  package_id: string | null;
  package_version: string | null;
  schema_version: string | null;
  title: string | null;
  references: ReferenceRecord[];
  diagnostics: ErrorView[];
}

export interface SourceEditorView {
  source_directory: string;
  package_id: string;
  editable: boolean;
  title: string;
  description: string | null;
  language: string;
  resource_title: string;
  markdown: string;
  concept_title: string;
}

/** Reference to either an existing schema ID or a package-generated pending ID. */
export type DraftKey =
  | { origin: "existing"; value: string }
  | { origin: "new"; value: string };

export interface ConceptDraft {
  key: DraftKey;
  title: string;
  requires: DraftKey[];
}

export interface ObjectiveDraft {
  key: DraftKey;
  concept: DraftKey;
  description: string;
}

export interface ResourceDraft {
  key: DraftKey;
  title: string;
  markdown: string;
  teaches: DraftKey[];
}

export interface CurriculumDraft {
  key: DraftKey;
  title: string;
  objectives: DraftKey[];
}

export interface OptionDraft {
  key: DraftKey;
  text: string;
}

export type AssessmentResponseDraft =
  | { type: "single_select"; options: OptionDraft[]; answer: DraftKey }
  | { type: "boolean"; answer: boolean };

export interface AssessmentDraft {
  key: DraftKey;
  measures: DraftKey[];
  stimulus: string;
  feedback: string;
  response: AssessmentResponseDraft;
}

export interface AuthoringWorkspace {
  sourceDirectory: string;
  packageId: string;
  editable: boolean;
  title: string;
  language: string;
  concepts: ConceptDraft[];
  objectives: ObjectiveDraft[];
  resources: ResourceDraft[];
  curricula: CurriculumDraft[];
  assessments: AssessmentDraft[];
}

export interface WorkspaceEdits extends Omit<AuthoringWorkspace, "packageId" | "editable"> {}

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
  /** Free-form, non-normative label for what the item asks of a learner. */
  cognitive_level?: string;
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
  references: ResourceReference[];
}

/**
 * Reference metadata already projected by Runtime to learner-visible fields.
 * Private records and hidden locators never reach this DTO.
 */
export interface ResourceReference {
  id: string;
  title: string;
  /** Legacy single-enum visibility, still emitted for older distributions. */
  visibility: "public" | "attribution_only" | "private";
  record_visibility?: "public" | "private";
  locator_visibility?: "public" | "hidden";
  citation?: string;
  locator?: string;
  type?: string;
  publisher?: string;
  authors?: string[];
  published_at?: string;
  updated_at?: string;
  accessed_at?: string;
  version?: string;
  edition?: string;
  identifiers?: Record<string, string>;
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
  /** Snapshot fields needed to interpret an event after its Package is removed. */
  assessment_snapshot?: {
    stimulus?: { markdown?: string };
  };
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
