// The only path from the renderer to the application: Tauri's own IPC.
//
// There is deliberately no `fetch`, no absolute filesystem path and no dynamic
// code loading in this module. Every call reaches a command in
// `src-tauri/src/commands.rs`, which forwards it to the shared application
// session. A command rejection carries the diagnostics Core and Package
// produced, so the UI can show the real reason.

import { invoke } from "@tauri-apps/api/core";
import type {
  AttemptView,
  AuthoringWorkspace,
  BuildReport,
  CommandError,
  ConceptSearchView,
  ErrorView,
  LessonView,
  ObjectiveProgress,
  PackageView,
  PackageContextView,
  ResourceView,
  StatusView,
  UninstallReport,
  SourceReviewView,
  SourceEditorView,
  WorkspaceEdits,
} from "./types.ts";

/** A command rejection, normalized so the UI never handles a raw value. */
export class OsmiumError extends Error {
  readonly diagnostics: ErrorView[];

  constructor(diagnostics: ErrorView[]) {
    super(
      diagnostics
        .map((diagnostic) => `${diagnostic.code}: ${diagnostic.message}`)
        .join("; ") || "the application returned no diagnostics",
    );
    this.name = "OsmiumError";
    this.diagnostics = diagnostics;
  }
}

function isNullableInteger(value: unknown): boolean {
  return value === null || (typeof value === "number" && Number.isInteger(value));
}

function isErrorView(value: unknown): value is ErrorView {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return typeof candidate.code === "string" &&
    typeof candidate.severity === "string" &&
    typeof candidate.message === "string" &&
    (candidate.file === null || typeof candidate.file === "string") &&
    isNullableInteger(candidate.line) &&
    isNullableInteger(candidate.column) &&
    typeof candidate.path === "string" &&
    Array.isArray(candidate.suggestions) &&
    candidate.suggestions.every((suggestion) => typeof suggestion === "string") &&
    (candidate.entity_type === undefined || typeof candidate.entity_type === "string") &&
    (candidate.entity_id === undefined || typeof candidate.entity_id === "string");
}

/** Normalize any rejection into an `OsmiumError`. */
export function toOsmiumError(rejection: unknown): OsmiumError {
  if (rejection instanceof OsmiumError) {
    return rejection;
  }
  if (typeof rejection === "object" && rejection !== null) {
    const diagnostics = (rejection as Partial<CommandError>).diagnostics;
    if (Array.isArray(diagnostics) && diagnostics.every(isErrorView)) {
      return new OsmiumError(diagnostics);
    }
  }
  const message =
    typeof rejection === "string"
      ? rejection
      : rejection instanceof Error
        ? rejection.message
        : "the application call failed";
  return new OsmiumError([
    { code: "OSM_SHELL", severity: "error", message, file: null, line: null, column: null, path: "", suggestions: [] },
  ]);
}

/** Invoke one shell command. */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (rejection) {
    throw toOsmiumError(rejection);
  }
}

/** The command surface, one function per Tauri command. */
export const osmium = {
  status: () => call<StatusView>("status"),
  defaultSourceDirectory: () => call<string>("default_source_directory"),
  reviewSource: (sourceDirectory: string) =>
    call<SourceReviewView>("review_source", { sourceDirectory }),
  createSource: (request: { directory: string; title: string; language: string }) =>
    call<SourceEditorView>("create_source", { request }),
  openSourceEditor: (sourceDirectory: string) =>
    call<SourceEditorView>("open_source_editor", { sourceDirectory }),
  saveSource: (request: SourceEditorView) =>
    call<SourceEditorView>("save_source", { request: { sourceDirectory: request.source_directory, title: request.title, language: request.language, resourceTitle: request.resource_title, conceptTitle: request.concept_title, markdown: request.markdown } }),
  openAuthoringWorkspace: (sourceDirectory: string) =>
    call<AuthoringWorkspace>("open_authoring_workspace", { sourceDirectory }),
  saveAuthoringWorkspace: (edits: WorkspaceEdits) =>
    call<AuthoringWorkspace>("save_authoring_workspace", { edits }),
  installSource: (sourceDirectory: string) =>
    call<{ package_id: string; package_version: string }>("install_source", { sourceDirectory }),
  installDistribution: (distributionPath: string) =>
    call<{ package_id: string; package_version: string }>("install_distribution", { distributionPath }),
  exportSource: (sourceDirectory: string, destination: string) =>
    call<BuildReport>("export_source", { sourceDirectory, destination }),
  uninstallPackage: (reference: { package_id: string; package_version: string }) =>
    call<UninstallReport>("uninstall_package", { packageId: reference.package_id, packageVersion: reference.package_version }),
  listPackages: () => call<PackageView[]>("list_packages"),
  openLesson: (packageId: string, version?: string) =>
    call<LessonView>("open_lesson", { packageId, version: version ?? null }),
  readResource: (packageId: string, resourceId: string, version?: string) =>
    call<ResourceView>("read_resource", {
      packageId,
      resourceId,
      version: version ?? null,
    }),
  submitAttempt: (
    packageId: string,
    assessmentId: string,
    response: string | boolean,
    options: { version?: string; requestId?: string; durationMs?: number } = {},
  ) =>
    call<AttemptView>("submit_attempt", {
      request: {
        packageId,
        assessmentId,
        response,
        version: options.version ?? null,
        requestId: options.requestId ?? null,
        durationMs: options.durationMs ?? null,
        hintsUsed: null,
      },
    }),
  progress: (packageId: string, version?: string) =>
    call<ObjectiveProgress[]>("progress", { packageId, version: version ?? null }),
  history: (packageId: string, version?: string, limit?: number, offset?: number) =>
    call<unknown[]>("history", {
      packageId,
      version: version ?? null,
      limit: limit ?? null,
      offset: offset ?? null,
    }),
  allHistory: (limit?: number, offset?: number) =>
    call<unknown[]>("all_history", { limit: limit ?? null, offset: offset ?? null }),
  packageContext: (
    packageId: string,
    kind: string,
    entityId: string,
    options: { version?: string; depth?: number; nodeLimit?: number } = {},
  ) => call<PackageContextView>("package_context", {
    packageId,
    version: options.version ?? null,
    kind,
    entityId,
    depth: options.depth ?? null,
    nodeLimit: options.nodeLimit ?? null,
  }),
  searchConcepts: (
    packageId: string,
    query: string,
    options: { version?: string; limit?: number } = {},
  ) => call<ConceptSearchView>("package_concept_search", {
    packageId,
    version: options.version ?? null,
    query,
    limit: options.limit ?? null,
  }),
  rebuildProgress: () => call<{ events_replayed: number }>("rebuild_progress"),
  exportState: (output: string) => call<{ events_exported: number }>("export_state", { output }),
  backupState: (output: string) => call<{ output: string }>("backup_state", { output }),
};
