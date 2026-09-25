// The application shell: navigation and state only.
//
// This file decides which panel is visible and holds the responses the shell
// already computed. It contains no grading, no validation and no version
// resolution: those live behind `osmium` commands.

import type { ReactElement } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AssessmentView } from "./components/Assessment.tsx";
import { Lesson } from "./components/Lesson.tsx";
import { PackageList } from "./components/Packages.tsx";
import { ProgressPanel } from "./components/ProgressPanel.tsx";
import { HistoryPanel } from "./components/HistoryPanel.tsx";
import { Authoring } from "./components/Authoring.tsx";
import { AppShell } from "./components/AppShell.tsx";
import { Reader } from "./components/Reader.tsx";
import { RoutePanel } from "./components/RoutePanel.tsx";
import { AtlasPanel } from "./components/AtlasPanel.tsx";
import { osmium, toOsmiumError } from "./client.ts";
import { readingOrder } from "./outline.ts";
import { defaultObjectiveId, objectiveForConcept } from "./learningNavigation.ts";
import { uiText, type UiLanguage } from "./i18n.ts";
import type {
  AttemptView,
  ConceptSearchView,
  ErrorView,
  InstalledPackageRef,
  LearningEvent,
  LessonView,
  ObjectiveProgress,
  PackageView,
  PackageContextView,
  Resource,
  ResourceView,
  StatusView,
  SourceReviewView,
} from "./types.ts";

type Panel =
  | { name: "packages" }
  | { name: "lesson" }
  | { name: "reader"; resourceId: string; returnTo: "lesson" | "route" | "atlas" | "assessment"; returnAssessmentId?: string }
  | { name: "assessment"; assessmentId: string; returnTo: "lesson" | "route" | "atlas" }
  | { name: "route" }
  | { name: "atlas"; conceptId: string }
  | { name: "progress" }
  | { name: "history" }
  | { name: "authoring" };

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
  const [packageLoadState, setPackageLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [packageLoadError, setPackageLoadError] = useState<ErrorView[]>([]);
  const [lesson, setLesson] = useState<LessonView | null>(null);
  const [resource, setResource] = useState<ResourceView | null>(null);
  const [progress, setProgress] = useState<ObjectiveProgress[]>([]);
  const [events, setEvents] = useState<LearningEvent[]>([]);
  const [sourceReview, setSourceReview] = useState<SourceReviewView | null>(null);
  const [authoringDirty, setAuthoringDirty] = useState(false);
  const [authoringLaunch, setAuthoringLaunch] = useState<"new" | "existing" | null>(null);
  const [installNotice, setInstallNotice] = useState(false);
  const [actionNotice, setActionNotice] = useState<string | null>(null);
  const [attempt, setAttempt] = useState<AttemptView | null>(null);
  const [focusedObjectiveId, setFocusedObjectiveId] = useState<string | null>(null);
  const [atlasContext, setAtlasContext] = useState<PackageContextView | null>(null);
  const [atlasConceptId, setAtlasConceptId] = useState<string | null>(null);
  const [atlasBusy, setAtlasBusy] = useState(false);
  const atlasRequest = useRef(0);
  const [panel, setPanel] = useState<Panel>({ name: "packages" });
  const [diagnostics, setDiagnostics] = useState<ErrorView[]>([]);
  const [busy, setBusy] = useState(false);
  const [scale, setScale] = useState(1);
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>(() =>
    localStorage.getItem("osmium.ui-language") === "en" ? "en" : "ja",
  );
  const t = (key: Parameters<typeof uiText>[1]) => uiText(uiLanguage, key);

  const fail = useCallback((error: unknown) => {
    setInstallNotice(false);
    setDiagnostics(toOsmiumError(error).diagnostics);
  }, []);

  const refreshPackages = useCallback(async () => {
    setBusy(true);
    setPackageLoadState((current) => current === "ready" ? current : "loading");
    try {
      setStatus(await osmium.status());
      setPackages(await osmium.listPackages());
      setPackageLoadError([]);
      setPackageLoadState("ready");
      setDiagnostics([]);
    } catch (error) {
      const normalized = toOsmiumError(error);
      setPackageLoadError(normalized.diagnostics);
      setPackageLoadState("error");
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
    async (reference: InstalledPackageRef) => {
      setBusy(true);
      try {
        const loaded = await osmium.openLesson(reference.package_id, reference.package_version);
        setLesson(loaded);
        setFocusedObjectiveId(defaultObjectiveId(loaded));
        atlasRequest.current += 1;
        setAtlasContext(null);
        setAtlasConceptId(null);
        setAtlasBusy(false);
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
    async (resourceId: string, returnTo: "lesson" | "route" | "atlas" | "assessment" = "lesson", returnAssessmentId?: string) => {
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
        const item = lesson.resources.find((candidate) => candidate.id === resourceId);
        const relatedObjective = returnTo === "assessment" && returnAssessmentId
          ? lesson.assessments.find((candidate) => candidate.id === returnAssessmentId)?.measures[0]
          : item?.teaches[0];
        if (relatedObjective) setFocusedObjectiveId(relatedObjective);
        if (returnTo === "assessment") setAttempt(null);
        setPanel({ name: "reader", resourceId, returnTo, ...(returnAssessmentId ? { returnAssessmentId } : {}) });
        setDiagnostics([]);
      } catch (error) {
        fail(error);
      } finally {
        setBusy(false);
      }
    },
    [fail, lesson],
  );

  const openAssessment = useCallback((assessmentId: string, returnTo: "lesson" | "route" | "atlas" = "lesson") => {
    const item = lesson?.assessments.find((candidate) => candidate.id === assessmentId);
    if (item?.measures[0]) setFocusedObjectiveId(item.measures[0]);
    setAttempt(null);
    setPanel({ name: "assessment", assessmentId, returnTo });
  }, [lesson]);

  const openRoute = useCallback((objectiveId?: string) => {
    if (objectiveId) setFocusedObjectiveId(objectiveId);
    else if (focusedObjectiveId === null && lesson !== null) setFocusedObjectiveId(defaultObjectiveId(lesson));
    setPanel({ name: "route" });
  }, [focusedObjectiveId, lesson]);

  const loadAtlasContext = useCallback(async (conceptId: string) => {
    if (lesson === null) return;
    const request = ++atlasRequest.current;
    setAtlasBusy(true);
    try {
      const context = await osmium.packageContext(lesson.package_id, "concept", conceptId, {
        version: lesson.package_version,
        depth: 3,
        nodeLimit: 64,
      });
      if (request === atlasRequest.current) {
        setAtlasContext(context);
        setDiagnostics([]);
      }
    } catch (error) {
      if (request === atlasRequest.current) {
        fail(error);
        setAtlasContext(null);
      }
    } finally {
      if (request === atlasRequest.current) setAtlasBusy(false);
    }
  }, [fail, lesson]);

  const openAtlas = useCallback((conceptId: string) => {
    setAtlasConceptId(conceptId);
    setPanel({ name: "atlas", conceptId });
    void loadAtlasContext(conceptId);
  }, [loadAtlasContext]);

  const searchConcepts = useCallback(async (query: string): Promise<ConceptSearchView> => {
    if (lesson === null) throw new Error("No Package is open");
    return await osmium.searchConcepts(lesson.package_id, query, {
      version: lesson.package_version,
      limit: 20,
    });
  }, [lesson]);

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
      setBusy(true);
      try {
        const page = (lesson === null
          ? await osmium.allHistory(HISTORY_PAGE, offset)
          : await osmium.history(
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

  const reviewSource = useCallback(async (sourceDirectory: string) => {
    setBusy(true);
    setSourceReview(null);
    try {
      setSourceReview(await osmium.reviewSource(sourceDirectory));
      setDiagnostics([]);
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }, [fail]);

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
  const returnToPanel = (returnTo: "lesson" | "route" | "atlas" | "assessment", returnAssessmentId?: string) => {
    if (returnTo === "route") setPanel({ name: "route" });
    else if (returnTo === "atlas" && atlasConceptId !== null) {
      setPanel({ name: "atlas", conceptId: atlasConceptId });
      void loadAtlasContext(atlasConceptId);
    } else if (returnTo === "assessment" && returnAssessmentId) {
      setAttempt(null);
      setPanel({ name: "assessment", assessmentId: returnAssessmentId, returnTo: "lesson" });
    } else setPanel({ name: "lesson" });
  };

  return (
    <AppShell
      section={panel.name}
      title={lesson?.manifest.title ?? null}
      scale={scale}
      onScale={setScale}
      busy={busy}
      status={status}
      onLibrary={async () => {
        if (panel.name === "authoring" && authoringDirty) {
          const discard = await confirm(
            uiLanguage === "ja" ? "保存していない変更があります。破棄してLibraryへ戻りますか？" : "You have unsaved changes. Discard them and return to the Library?",
            { title: uiLanguage === "ja" ? "未保存の変更" : "Unsaved changes", kind: "warning" },
          );
          if (!discard) return;
        }
        setAuthoringDirty(false);
        setPanel({ name: "packages" });
        void refreshPackages();
      }}
      onLesson={() => setPanel({ name: "lesson" })}
      onProgress={() => void showProgress()}
      onHistory={() => void showHistory(0)}
      onAuthoring={() => {
        setPanel({ name: "authoring" });
        setDiagnostics([]);
      }}
      language={uiLanguage}
      onLanguage={(language) => {
        localStorage.setItem("osmium.ui-language", language);
        setUiLanguage(language);
      }}
    >
      {diagnostics.length > 0 ? (
        <div className="error" role="alert">
          <DiagnosticList diagnostics={diagnostics} />
          <button type="button" onClick={() => setDiagnostics([])}>
            {t("close")}
          </button>
        </div>
      ) : null}
      {installNotice ? <p className="notice" role="status">{uiLanguage === "ja" ? "教材をライブラリに追加し、学習画面を開きました。" : "Course added to your Library and opened in learner view."}</p> : null}
      {actionNotice ? <p className="notice" role="status">{actionNotice}</p> : null}

      {panel.name === "packages" ? (
        <PackageList
          packages={packages}
          selected={lesson === null ? null : { package_id: lesson.package_id, package_version: lesson.package_version }}
          onSelect={(reference) => void openPackage(reference)}
          onRefresh={() => void refreshPackages()}
          busy={busy}
          loadState={packageLoadState}
          loadError={packageLoadError}
          onCreate={() => { setAuthoringLaunch("new"); setPanel({ name: "authoring" }); }}
          onOpenSource={() => { setAuthoringLaunch("existing"); setPanel({ name: "authoring" }); }}
          onImport={() => void (async () => {
            try {
              const testPath = import.meta.env.VITE_OSMIUM_E2E === "1" ? window.osmiumE2eDistributionPath : undefined;
              const selected = typeof testPath === "string" ? testPath : await open({ multiple: false, directory: false, filters: [{ name: "CosmoOrder course", extensions: ["osmium"] }] });
              if (typeof selected !== "string") return;
              setBusy(true);
              const installed = await osmium.installDistribution(selected);
              await refreshPackages();
              await openPackage(installed);
              setInstallNotice(true);
            } catch (error) {
              fail(error);
            } finally {
              setBusy(false);
            }
          })()}
          onRemove={(reference) => void (async () => {
            const title = packages.find((item) => item.package_id === reference.package_id && item.package_version === reference.package_version)?.title ?? reference.package_id;
            const testDecision = import.meta.env.VITE_OSMIUM_E2E === "1" ? window.osmiumE2eConfirmRemoval : undefined;
            const accepted = typeof testDecision === "boolean" ? testDecision : await confirm(
              uiLanguage === "ja" ? `「${title}」v${reference.package_version}をライブラリから削除しますか？学習履歴は保持されます。` : `Remove “${title}” v${reference.package_version} from your Library? Learning history will be kept.`,
              { title: uiLanguage === "ja" ? "教材を削除" : "Remove material", kind: "warning" },
            );
            if (!accepted) return;
            try {
              setActionNotice(null);
              setBusy(true);
              const removed = await osmium.uninstallPackage(reference);
              if (removed.package_id !== reference.package_id || removed.package_version !== reference.package_version) {
                throw new Error(uiLanguage === "ja" ? "削除結果が選択した教材と一致しません。ライブラリを再読み込みして確認してください。" : "The removal report did not match the selected course. Refresh the Library to verify its state.");
              }
              if (lesson?.package_id === reference.package_id && lesson.package_version === reference.package_version) {
                setLesson(null);
                setResource(null);
                setAttempt(null);
                setProgress([]);
                setEvents([]);
                setFocusedObjectiveId(null);
                setAtlasConceptId(null);
                setAtlasContext(null);
                atlasRequest.current += 1;
                setPanel({ name: "packages" });
              }
              setInstallNotice(false);
              await refreshPackages();
              setActionNotice(uiLanguage === "ja" ? `「${title}」v${reference.package_version}を削除しました。学習履歴は保持されています。` : `Removed “${title}” v${reference.package_version}. Learning history is retained.`);
            } catch (error) {
              fail(error);
            } finally {
              setBusy(false);
            }
          })()}
        />
      ) : null}

      {panel.name === "authoring" ? (
        <Authoring review={sourceReview} busy={busy} launch={authoringLaunch} onLaunchHandled={() => setAuthoringLaunch(null)} onReview={reviewSource} onClearReview={() => setSourceReview(null)} onError={fail} onDirtyChange={setAuthoringDirty} onInstalled={async (reference) => { setInstallNotice(true); setAuthoringDirty(false); await refreshPackages(); await openPackage(reference); }} />
      ) : null}

      {lesson !== null && panel.name === "lesson" ? (
        <Lesson
          key={lesson.digest}
          busy={busy}
          lesson={lesson}
          progress={progress}
          currentObjectiveId={focusedObjectiveId}
          onOpenResource={(resourceId) => void openResource(resourceId)}
          onOpenAssessment={openAssessment}
          onOpenAtlas={openAtlas}
          onOpenObjective={(objectiveId) => {
            setFocusedObjectiveId(objectiveId);
            const objectiveResource = lesson.resources.find((item) => item.teaches.includes(objectiveId));
            const assessment = lesson.assessments.find((item) => item.measures.includes(objectiveId));
            if (objectiveResource) void openResource(objectiveResource.id);
            else if (assessment) openAssessment(assessment.id);
          }}
          onOpenRoute={() => openRoute()}
        />
      ) : null}

      {lesson !== null && panel.name === "route" ? (
        <RoutePanel
          lesson={lesson}
          objectiveId={focusedObjectiveId}
          progress={progress}
          busy={busy || atlasBusy}
          onSelectObjective={setFocusedObjectiveId}
          onOpenAtlas={openAtlas}
          onOpenResource={(resourceId) => void openResource(resourceId, "route")}
          onBack={() => setPanel({ name: "lesson" })}
        />
      ) : null}

      {lesson !== null && panel.name === "atlas" ? (
        <AtlasPanel
          lesson={lesson}
          selectedConceptId={panel.conceptId}
          context={atlasContext}
          progress={progress}
          busy={busy || atlasBusy}
          onSearch={searchConcepts}
          onRecenter={(conceptId) => { setAtlasConceptId(conceptId); setPanel({ name: "atlas", conceptId }); void loadAtlasContext(conceptId); }}
          onStudyConcept={(conceptId) => {
            const objectiveId = objectiveForConcept(lesson, conceptId);
            if (objectiveId) {
              setFocusedObjectiveId(objectiveId);
              setPanel({ name: "route" });
            } else {
              setPanel({ name: "lesson" });
            }
          }}
          onOpenResource={(resourceId) => void openResource(resourceId, "atlas")}
          onOpenAssessment={(assessmentId) => openAssessment(assessmentId, "atlas")}
          onBackToRoute={() => setPanel({ name: "route" })}
          onBackToLesson={() => setPanel({ name: "lesson" })}
        />
      ) : null}

      {lesson !== null && selectedResource !== null ? (
        <Reader
          key={selectedResource.id}
          courseTitle={lesson.manifest.title}
          packageId={lesson.package_id}
          resource={selectedResource}
          view={resource}
          previous={previous}
          next={next}
          loading={busy}
          onNavigate={(resourceId) => void openResource(resourceId, panel.name === "reader" ? panel.returnTo : "lesson", panel.name === "reader" ? panel.returnAssessmentId : undefined)}
          onBack={() => returnToPanel(panel.name === "reader" ? panel.returnTo : "lesson", panel.name === "reader" ? panel.returnAssessmentId : undefined)}
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
          onBack={() => returnToPanel(panel.name === "assessment" ? panel.returnTo : "lesson")}
          onNext={() => {
            if (nextAssessment !== null) {
              openAssessment(nextAssessment.id, panel.name === "assessment" ? panel.returnTo : "lesson");
            }
          }}
          hasNext={nextAssessment !== null}
          {...(attempt && !attempt.correct ? { onReviewResource: () => {
            const reviewResource = lesson.resources.find((candidate) => candidate.teaches.some((objectiveId) => currentAssessment.measures.includes(objectiveId)));
            if (reviewResource) void openResource(reviewResource.id, "assessment", currentAssessment.id);
          } } : {})}
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
          showPackageIdentity={false}
          onLoadMore={() => void showHistory(events.length)}
          onBack={() => setPanel({ name: "lesson" })}
          busy={busy}
        />
      ) : null}
      {lesson === null && panel.name === "history" ? (
        <HistoryPanel
          stimuli={{}}
          events={events}
          objectives={new Map()}
          showPackageIdentity
          onLoadMore={() => void showHistory(events.length)}
          onBack={() => setPanel({ name: "packages" })}
          busy={busy}
        />
      ) : null}
    </AppShell>
  );
}
