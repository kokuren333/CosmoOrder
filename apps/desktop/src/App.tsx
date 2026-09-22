// The application shell: navigation and state only.
//
// This file decides which panel is visible and holds the responses the shell
// already computed. It contains no grading, no validation and no version
// resolution: those live behind `osmium` commands.

import type { ReactElement } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { AssessmentView } from "./components/Assessment.tsx";
import { Lesson } from "./components/Lesson.tsx";
import { PackageList } from "./components/Packages.tsx";
import { ProgressPanel } from "./components/ProgressPanel.tsx";
import { HistoryPanel } from "./components/HistoryPanel.tsx";
import { AppShell } from "./components/AppShell.tsx";
import { Reader } from "./components/Reader.tsx";
import { osmium, toOsmiumError } from "./client.ts";
import { readingOrder } from "./outline.ts";
import type {
  AttemptView,
  ErrorView,
  LearningEvent,
  LessonView,
  ObjectiveProgress,
  PackageView,
  Resource,
  ResourceView,
  StatusView,
} from "./types.ts";

type Panel =
  | { name: "packages" }
  | { name: "lesson" }
  | { name: "reader"; resourceId: string }
  | { name: "assessment"; assessmentId: string }
  | { name: "progress" }
  | { name: "history" };

const HISTORY_PAGE = 16;

function DiagnosticList({
  diagnostics,
}: {
  diagnostics: ErrorView[];
}): ReactElement {
  return (
    <ul className="diagnostics">
      {diagnostics.map((diagnostic, index) => (
        <li key={`${diagnostic.code}-${index}`}>
          <strong>{diagnostic.code}</strong> {diagnostic.message}
          {diagnostic.file === null ? null : (
            <>
              {" "}
              <code>{diagnostic.file}</code>
            </>
          )}
          {diagnostic.suggestions.length === 0 ? null : (
            <ul>
              {diagnostic.suggestions.map((suggestion) => (
                <li key={suggestion}>{suggestion}</li>
              ))}
            </ul>
          )}
        </li>
      ))}
    </ul>
  );
}

export function App(): ReactElement {
  const [status, setStatus] = useState<StatusView | null>(null);
  const [packages, setPackages] = useState<PackageView[]>([]);
  const [lesson, setLesson] = useState<LessonView | null>(null);
  const [resource, setResource] = useState<ResourceView | null>(null);
  const [progress, setProgress] = useState<ObjectiveProgress[]>([]);
  const [events, setEvents] = useState<LearningEvent[]>([]);
  const [attempt, setAttempt] = useState<AttemptView | null>(null);
  const [panel, setPanel] = useState<Panel>({ name: "packages" });
  const [diagnostics, setDiagnostics] = useState<ErrorView[]>([]);
  const [busy, setBusy] = useState(false);
  const [scale, setScale] = useState(1);

  const fail = useCallback((error: unknown) => {
    setDiagnostics(toOsmiumError(error).diagnostics);
  }, []);

  const refreshPackages = useCallback(async () => {
    setBusy(true);
    try {
      setStatus(await osmium.status());
      setPackages(await osmium.listPackages());
      setDiagnostics([]);
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }, [fail]);

  useEffect(() => {
    void refreshPackages();
  }, [refreshPackages]);

  const pageKey =
    panel.name === "reader"
      ? `reader:${panel.resourceId}`
      : panel.name === "assessment"
        ? `assessment:${panel.assessmentId}`
        : panel.name;
  useEffect(() => {
    document.getElementById("main-content")?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }, [pageKey]);

  const objectives = useMemo(() => {
    const map = new Map<string, string>();
    for (const objective of lesson?.objectives ?? []) {
      map.set(objective.id, objective.description);
    }
    return map;
  }, [lesson]);

  const openPackage = useCallback(
    async (packageId: string) => {
      setBusy(true);
      try {
        const loaded = await osmium.openLesson(packageId);
        setLesson(loaded);
        setResource(null);
        setAttempt(null);
        setEvents([]);
        setProgress(
          await osmium.progress(loaded.package_id, loaded.package_version),
        );
        setPanel({ name: "lesson" });
        setDiagnostics([]);
      } catch (error) {
        fail(error);
      } finally {
        setBusy(false);
      }
    },
    [fail],
  );

  const openResource = useCallback(
    async (resourceId: string) => {
      if (lesson === null) {
        return;
      }
      setBusy(true);
      try {
        setResource(
          await osmium.readResource(
            lesson.package_id,
            resourceId,
            lesson.package_version,
          ),
        );
        setPanel({ name: "reader", resourceId });
        setDiagnostics([]);
      } catch (error) {
        fail(error);
      } finally {
        setBusy(false);
      }
    },
    [fail, lesson],
  );

  const openAssessment = useCallback((assessmentId: string) => {
    setAttempt(null);
    setPanel({ name: "assessment", assessmentId });
  }, []);

  const answer = useCallback(
    async (assessmentId: string, response: string | boolean) => {
      if (lesson === null) {
        return;
      }
      setBusy(true);
      try {
        const result = await osmium.submitAttempt(
          lesson.package_id,
          assessmentId,
          response,
          { version: lesson.package_version },
        );
        setAttempt(result);
        setProgress(
          await osmium.progress(lesson.package_id, lesson.package_version),
        );
        setDiagnostics([]);
      } catch (error) {
        fail(error);
      } finally {
        setBusy(false);
      }
    },
    [fail, lesson],
  );

  const showProgress = useCallback(async () => {
    if (lesson === null) {
      return;
    }
    setBusy(true);
    try {
      setProgress(
        await osmium.progress(lesson.package_id, lesson.package_version),
      );
      setPanel({ name: "progress" });
      setDiagnostics([]);
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }, [fail, lesson]);

  const showHistory = useCallback(
    async (offset = 0) => {
      if (lesson === null) {
        return;
      }
      setBusy(true);
      try {
        const page = (await osmium.history(
          lesson.package_id,
          lesson.package_version,
          HISTORY_PAGE,
          offset,
        )) as LearningEvent[];
        setEvents((current) => (offset === 0 ? page : [...current, ...page]));
        setPanel({ name: "history" });
        setDiagnostics([]);
      } catch (error) {
        fail(error);
      } finally {
        setBusy(false);
      }
    },
    [fail, lesson],
  );

  const rebuild = useCallback(async () => {
    setBusy(true);
    try {
      await osmium.rebuildProgress();
      if (lesson !== null) {
        setProgress(
          await osmium.progress(lesson.package_id, lesson.package_version),
        );
      }
      setDiagnostics([]);
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }, [fail, lesson]);

  const order: Resource[] = useMemo(
    () => (lesson === null ? [] : readingOrder(lesson)),
    [lesson],
  );

  const selectedResource =
    panel.name === "reader"
      ? (order.find((item) => item.id === panel.resourceId) ?? null)
      : null;
  const resourceIndex =
    selectedResource === null
      ? -1
      : order.findIndex((item) => item.id === selectedResource.id);
  const previous =
    resourceIndex > 0 ? (order[resourceIndex - 1] ?? null) : null;
  const next =
    resourceIndex >= 0 && resourceIndex < order.length - 1
      ? (order[resourceIndex + 1] ?? null)
      : null;

  const currentAssessment =
    panel.name === "assessment"
      ? (lesson?.assessments.find((item) => item.id === panel.assessmentId) ??
        null)
      : null;
  const assessmentIndex =
    currentAssessment === null
      ? -1
      : (lesson?.assessments.findIndex(
          (item) => item.id === currentAssessment.id,
        ) ?? -1);
  const nextAssessment =
    lesson !== null &&
    assessmentIndex >= 0 &&
    assessmentIndex < lesson.assessments.length - 1
      ? (lesson.assessments[assessmentIndex + 1] ?? null)
      : null;

  return (
    <AppShell
      section={panel.name}
      title={lesson?.manifest.title ?? null}
      scale={scale}
      onScale={setScale}
      busy={busy}
      status={status}
      onLibrary={() => {
        setPanel({ name: "packages" });
        void refreshPackages();
      }}
      onLesson={() => setPanel({ name: "lesson" })}
      onProgress={() => void showProgress()}
      onHistory={() => void showHistory(0)}
    >
      {diagnostics.length > 0 ? (
        <div className="error" role="alert">
          <DiagnosticList diagnostics={diagnostics} />
          <button type="button" onClick={() => setDiagnostics([])}>
            閉じる
          </button>
        </div>
      ) : null}

      {panel.name === "packages" || lesson === null ? (
        <PackageList
          packages={packages}
          selected={lesson?.package_id ?? null}
          onSelect={(packageId) => void openPackage(packageId)}
          onRefresh={() => void refreshPackages()}
          busy={busy}
        />
      ) : null}

      {lesson !== null && panel.name === "lesson" ? (
        <Lesson
          key={lesson.digest}
          busy={busy}
          lesson={lesson}
          progress={progress}
          onOpenResource={(resourceId) => void openResource(resourceId)}
          onOpenAssessment={openAssessment}
        />
      ) : null}

      {lesson !== null && selectedResource !== null ? (
        <Reader
          key={selectedResource.id}
          courseTitle={lesson.manifest.title}
          resource={selectedResource}
          view={resource}
          previous={previous}
          next={next}
          loading={busy}
          onNavigate={(resourceId) => void openResource(resourceId)}
          onBack={() => setPanel({ name: "lesson" })}
        />
      ) : null}

      {lesson !== null && currentAssessment !== null ? (
        <AssessmentView
          key={currentAssessment.id}
          position={assessmentIndex + 1}
          total={lesson.assessments.length}
          onShowProgress={() => void showProgress()}
          assessment={currentAssessment}
          stimulus={lesson.stimuli[currentAssessment.id] ?? null}
          attempt={attempt}
          busy={busy}
          onAnswer={(response) => void answer(currentAssessment.id, response)}
          onBack={() => setPanel({ name: "lesson" })}
          onNext={() => {
            if (nextAssessment !== null) {
              openAssessment(nextAssessment.id);
            }
          }}
          hasNext={nextAssessment !== null}
        />
      ) : null}

      {lesson !== null && panel.name === "progress" ? (
        <ProgressPanel
          progress={progress}
          objectives={objectives}
          onRebuild={() => void rebuild()}
          onBack={() => setPanel({ name: "lesson" })}
          busy={busy}
        />
      ) : null}

      {lesson !== null && panel.name === "history" ? (
        <HistoryPanel
          stimuli={lesson.stimuli}
          events={events}
          objectives={objectives}
          onLoadMore={() => void showHistory(events.length)}
          onBack={() => setPanel({ name: "lesson" })}
          busy={busy}
        />
      ) : null}
    </AppShell>
  );
}
