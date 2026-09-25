import { useEffect, useState } from "react";
import { open, save as saveFile, confirm } from "@tauri-apps/plugin-dialog";
import { ArrowDown, ArrowUp, Copy, Download, FolderOpen, Plus, Save, ShieldCheck, Trash2 } from "lucide-react";
import type {
  AssessmentDraft, AssessmentResponseDraft, AuthoringWorkspace,
  DraftKey, InstalledPackageRef, SourceReviewView,
} from "../types.ts";
import { localize, useUiLanguage } from "../i18n.ts";
import { osmium } from "../client.ts";
import { AuthoringReview } from "./AuthoringReview.tsx";

type Operation = "creating" | "opening" | "choosing-directory" | "saving" | "validating" | "installing" | null;
const COMMON_LANGUAGES = [
  ["ja", "日本語"], ["en", "English"], ["zh", "中文"], ["es", "Español"], ["fr", "Français"],
  ["de", "Deutsch"], ["ko", "한국어"], ["pt", "Português"], ["hi", "हिन्दी"], ["ar", "العربية"],
] as const;

const newKey = (): DraftKey => ({ origin: "new", value: crypto.randomUUID().replaceAll("-", "") });
const token = (key: DraftKey) => `${key.origin}:${key.value}`;
const fromToken = (value: string): DraftKey => {
  const separator = value.indexOf(":");
  return { origin: value.slice(0, separator) === "new" ? "new" : "existing", value: value.slice(separator + 1) };
};
const sameKey = (left: DraftKey, right: DraftKey) => token(left) === token(right);
const selected = (items: DraftKey[], key: DraftKey) => items.some((item) => sameKey(item, key));
const localizedEntityName = (kind: "concept" | "objective" | "resource" | "curriculum" | "assessment", index: number, english: boolean) => {
  const names = english
    ? { concept: "Topic", objective: "Learning goal", resource: "Learning content", curriculum: "Learning path", assessment: "Question" }
    : { concept: "テーマ", objective: "できるようになること", resource: "教材コンテンツ", curriculum: "学習順序", assessment: "問題" };
  return `${names[kind]} ${index + 1}`;
};

function updateMany(items: DraftKey[], key: DraftKey, checked: boolean): DraftKey[] {
  if (checked) return selected(items, key) ? items : [...items, key];
  return items.filter((item) => !sameKey(item, key));
}

export function Authoring({
  review, busy, launch, onLaunchHandled, onReview, onClearReview, onError, onInstalled, onDirtyChange,
}: {
  review: SourceReviewView | null;
  busy: boolean;
  launch: "new" | "existing" | null;
  onLaunchHandled: () => void;
  onReview: (sourceDirectory: string) => Promise<void>;
  onClearReview: () => void;
  onError: (error: unknown) => void;
  onInstalled: (reference: InstalledPackageRef) => Promise<void>;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const language = useUiLanguage();
  const english = language === "en";
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const [workspace, setWorkspace] = useState<AuthoringWorkspace | null>(null);
  const [dirty, setDirty] = useState(false);
  const [creating, setCreating] = useState(false);
  const [operation, setOperation] = useState<Operation>(null);
  const [newTitle, setNewTitle] = useState("");
  const [newLanguage, setNewLanguage] = useState(language === "en" ? "en" : "ja");
  const [newDirectory, setNewDirectory] = useState("");
  const [exportedPath, setExportedPath] = useState<string | null>(null);
  const working = busy || operation !== null;
  const hasErrors = review !== null && (!review.valid || review.diagnostics.some((item) => item.severity === "error"));
  const hasWarnings = review?.diagnostics.some((item) => item.severity === "warning") ?? false;
  const validationState = review === null
    ? tr("未チェック", "Not checked")
    : dirty
      ? tr("変更後に未チェック", "Changed since last check")
      : hasErrors
        ? tr("エラーあり", "Errors")
        : hasWarnings
          ? tr("注意点あり", "Warnings")
          : tr("チェック済み", "Checked");
  const sourceFolderLabel = newDirectory.split(/[\\/]/).filter(Boolean).slice(-3).join(" / ");

  useEffect(() => {
    void osmium.defaultSourceDirectory().then(setNewDirectory).catch(onError);
  }, [onError]);

  const markDirty = (next: AuthoringWorkspace) => {
    setWorkspace(next);
    setDirty(true);
    onDirtyChange(true);
  };

  const mayDiscard = async () => !dirty || await confirm(
    tr("保存していない変更があります。破棄しますか？", "Discard unsaved changes?"),
    { title: tr("未保存の変更", "Unsaved changes"), kind: "warning" },
  );

  const chooseSource = async () => {
    if (!(await mayDiscard())) return;
    setOperation("opening");
    try {
      const testPath = import.meta.env.VITE_OSMIUM_E2E === "1" ? window.osmiumE2eSourceDirectory : undefined;
      const chosen = typeof testPath === "string" ? testPath : await open({ directory: true, multiple: false, title: tr("編集用教材フォルダーを選択", "Select an editing folder") });
      if (typeof chosen !== "string") return;
      setWorkspace(null);
      setDirty(false);
      onDirtyChange(false);
      await onReview(chosen);
      const loaded = await osmium.openAuthoringWorkspace(chosen);
      setWorkspace(loaded);
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  useEffect(() => {
    if (launch === null) return;
    onLaunchHandled();
    if (launch === "new") setCreating(true);
    else void chooseSource();
  }, [launch]);

  const chooseNewDirectory = async () => {
    setOperation("choosing-directory");
    try {
      const chosen = await open({ directory: true, multiple: false, ...(newDirectory ? { defaultPath: newDirectory } : {}), title: tr("教材の保存先を選択", "Choose where to save courses") });
      if (typeof chosen === "string") setNewDirectory(chosen);
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  const createSource = async () => {
    setOperation("creating");
    try {
      const testDirectory = import.meta.env.VITE_OSMIUM_E2E === "1" ? window.osmiumE2eNewSourceDirectory : undefined;
      const created = await osmium.createSource({ directory: testDirectory ?? newDirectory, title: newTitle.trim(), language: newLanguage });
      const loaded = await osmium.openAuthoringWorkspace(created.source_directory);
      setWorkspace(loaded);
      setDirty(false);
      onDirtyChange(false);
      setCreating(false);
      onClearReview();
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  const save = async () => {
    if (!workspace || !workspace.editable || !dirty) return;
    setOperation("saving");
    try {
      const saved = await osmium.saveAuthoringWorkspace({
        sourceDirectory: workspace.sourceDirectory,
        title: workspace.title,
        language: workspace.language,
        concepts: workspace.concepts,
        objectives: workspace.objectives,
        resources: workspace.resources,
        curricula: workspace.curricula,
        assessments: workspace.assessments,
      });
      setWorkspace(saved);
      setDirty(false);
      onDirtyChange(false);
      onClearReview();
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  const check = async () => {
    if (!workspace || dirty) return;
    setOperation("validating");
    try {
      await onReview(workspace.sourceDirectory);
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  const install = async () => {
    if (!workspace?.editable || dirty || !review?.valid || hasErrors) return;
    setOperation("installing");
    try {
      // This Package command rebuilds and Core-validates the current Source,
      // then installs only the verified Distribution via the shared Runtime.
      const installed = await osmium.installSource(workspace.sourceDirectory);
      await onInstalled({ package_id: installed.package_id, package_version: installed.package_version });
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  const exportDistribution = async () => {
    if (!workspace?.editable || dirty) return;
    try {
      const testPath = import.meta.env.VITE_OSMIUM_E2E === "1" ? window.osmiumE2eExportDestination : undefined;
      const destination = typeof testPath === "string" ? testPath : await saveFile({
        defaultPath: `${workspace.title.replace(/[\\/:*?"<>|]/g, "-") || "course"}.osmium`,
        filters: [{ name: "CosmoOrder course", extensions: ["osmium"] }],
        title: tr("教材ファイルを書き出す", "Export course file"),
      });
      if (typeof destination !== "string") return;
      setOperation("validating");
      const report = await osmium.exportSource(workspace.sourceDirectory, destination);
      setExportedPath(report.output);
    } catch (error) {
      onError(error);
    } finally {
      setOperation(null);
    }
  };

  const conceptName = (key: DraftKey) => {
    const index = workspace?.concepts.findIndex((item) => sameKey(item.key, key)) ?? -1;
    return index < 0 ? tr("削除されたテーマ", "Deleted topic") : localizedEntityName("concept", index, english);
  };
  const objectiveName = (key: DraftKey) => {
    const index = workspace?.objectives.findIndex((item) => sameKey(item.key, key)) ?? -1;
    const objective = index < 0 ? null : workspace?.objectives[index];
    return objective ? `${localizedEntityName("objective", index, english)} — ${objective.description || tr("名前未設定", "Unnamed")}` : tr("削除された学習目標", "Deleted learning goal");
  };
  const addConcept = () => workspace && markDirty({ ...workspace, concepts: [...workspace.concepts, { key: newKey(), title: "", requires: [] }] });
  const addObjective = () => workspace && workspace.concepts.length > 0 && markDirty({ ...workspace, objectives: [...workspace.objectives, { key: newKey(), concept: workspace.concepts[0]!.key, description: "" }] });
  const addResource = () => workspace && workspace.objectives.length > 0 && markDirty({ ...workspace, resources: [...workspace.resources, { key: newKey(), title: "", markdown: "", teaches: [workspace.objectives[0]!.key] }] });
  const addCurriculum = () => workspace && workspace.objectives.length > 0 && markDirty({ ...workspace, curricula: [...workspace.curricula, { key: newKey(), title: "", objectives: [workspace.objectives[0]!.key] }] });
  const addAssessment = (type: "single_select" | "boolean") => {
    if (!workspace || workspace.objectives.length === 0) return;
    const response: AssessmentResponseDraft = type === "boolean"
      ? { type, answer: true }
      : { type, options: [{ key: newKey(), text: "" }, { key: newKey(), text: "" }], answer: newKey() };
    if (response.type === "single_select") response.answer = response.options[0]!.key;
    const item: AssessmentDraft = { key: newKey(), measures: [workspace.objectives[0]!.key], stimulus: "", feedback: "", response };
    markDirty({ ...workspace, assessments: [...workspace.assessments, item] });
  };

  return (
    <section className="authoring" aria-labelledby="authoring-heading">
      <header className="panel-head">
        <div>
          <p className="eyebrow">{tr("教材をつくる", "CREATE MATERIAL")}</p>
          <h1 id="authoring-heading">{tr("教材作成", "Authoring")}</h1>
          <p className="lede">{tr("学ぶ内容、できるようになること、教材コンテンツを組み立てます。", "Build topics, learning goals, and learning content into a course.")}</p>
        </div>
        <div className="authoring-actions">
          <button type="button" className="primary" onClick={async () => { if (await mayDiscard()) setCreating(true); }} disabled={working}>
            <Plus size={17} aria-hidden="true" />{tr("新しい教材を作成", "New material")}
          </button>
          <button type="button" onClick={() => void chooseSource()} disabled={working}>
            <FolderOpen size={17} aria-hidden="true" />{operation === "opening" ? tr("読み込み中…", "Opening…") : tr("編集用フォルダーを開く", "Open editing folder")}
          </button>
        </div>
      </header>
      {exportedPath ? <p className="notice" role="status">{tr("教材ファイルを書き出しました", "Course file exported")}: <code>{exportedPath}</code></p> : null}

      {creating ? (
        <section className="card authoring-form" aria-labelledby="new-source-heading">
          <h2 id="new-source-heading">{tr("新しい教材", "New material")}</h2>
          <label>{tr("教材名", "Course name")}<input autoFocus value={newTitle} onChange={(event) => setNewTitle(event.target.value)} /></label>
          <label>{tr("言語", "Language")}
            <select value={COMMON_LANGUAGES.some(([code]) => code === newLanguage) ? newLanguage : "other"} onChange={(event) => setNewLanguage(event.target.value === "other" ? "" : event.target.value)}>
              {COMMON_LANGUAGES.map(([code, name]) => <option key={code} value={code}>{name}</option>)}
              <option value="other">{tr("その他…", "Other…")}</option>
            </select>
            {!COMMON_LANGUAGES.some(([code]) => code === newLanguage) ? <input aria-label={tr("言語コード", "Language code")} placeholder="it, zh-Hans" value={newLanguage} onChange={(event) => setNewLanguage(event.target.value)} /> : null}
          </label>
          <label>{tr("保存先", "Save location")}
            <div className="folder-picker-row">
              <span className="folder-location"><FolderOpen size={17} aria-hidden="true" />{sourceFolderLabel || tr("保存先を読み込み中…", "Loading save location…")}</span>
              <button type="button" onClick={() => void chooseNewDirectory()} disabled={working}>{operation === "choosing-directory" ? tr("選択中…", "Choosing…") : tr("変更", "Change")}</button>
            </div>
          </label>
          <details className="authoring-internals">
            <summary>{tr("詳細設定", "Advanced")}</summary>
            <dl><dt>Package ID</dt><dd>{tr("作成時に自動生成", "Generated when created")}</dd><dt>{tr("言語コード", "Language code")}</dt><dd>{newLanguage || "—"}</dd><dt>{tr("保存先の実パス", "Absolute save location")}</dt><dd><code>{newDirectory || "—"}</code></dd></dl>
          </details>
          <div className="authoring-actions">
            <button type="button" onClick={() => setCreating(false)} disabled={working}>{tr("キャンセル", "Cancel")}</button>
            <button className="primary" disabled={working || !newTitle.trim() || !newLanguage.trim() || !newDirectory.trim()} onClick={() => void createSource()}>{operation === "creating" ? tr("作成中…", "Creating…") : tr("作成", "Create")}</button>
          </div>
        </section>
      ) : null}

      {workspace ? (
        <>
          {!workspace.editable ? <div className="notice" role="status"><strong>{tr("YAML教材は表示のみです", "YAML course is read-only")}</strong><p>{tr("編集と保存にはJSON形式の編集用フォルダーを開いてください。", "Open a JSON editing folder to make and save changes.")}</p></div> : null}
          <section className="source-review-summary" data-valid={review?.valid ?? false}>
            <h2>{workspace.title}</h2>
            <details className="authoring-internals"><summary>{tr("詳細設定", "Advanced")}</summary><dl>
              <dt>Package ID</dt><dd><code>{workspace.packageId}</code> <button type="button" aria-label={tr("Package IDをコピー", "Copy Package ID")} onClick={() => void navigator.clipboard.writeText(workspace.packageId).catch(onError)}><Copy size={15} aria-hidden="true" />{tr("コピー", "Copy")}</button></dd>
              <dt>{tr("言語コード", "Language code")}</dt><dd><code>{workspace.language}</code></dd>
              <dt>{tr("編集用フォルダーの実パス", "Absolute editing folder")}</dt><dd><code>{workspace.sourceDirectory}</code></dd>
            </dl></details>
            <p role="status">{tr("教材の状態", "Course status")}: <strong>{validationState}</strong></p>
            {dirty ? <p role="status">{tr("未保存の変更があります。保存してからチェックしてください。", "Unsaved changes. Save before checking or adding this material.")}</p> : null}
            <div className="authoring-actions">
              <button onClick={() => void save()} disabled={working || !workspace.editable || !dirty}><Save size={16} aria-hidden="true" />{operation === "saving" ? tr("保存中…", "Saving…") : tr("保存", "Save")}</button>
              <button onClick={() => void check()} disabled={working || dirty}><ShieldCheck size={16} aria-hidden="true" />{operation === "validating" ? tr("確認中…", "Checking…") : tr("教材をチェック", "Check material")}</button>
              <button onClick={() => void exportDistribution()} disabled={working || !workspace.editable || dirty}><Download size={16} aria-hidden="true" />{tr("教材ファイルを書き出す", "Export course file")}</button>
              <button className="primary" onClick={() => void install()} disabled={working || !workspace.editable || dirty || review === null || hasErrors}><Download size={16} aria-hidden="true" />{operation === "installing" ? tr("追加中…", "Adding…") : tr("ライブラリに追加して学ぶ", "Add to Library and learn")}</button>
            </div>
          </section>

          <section className="card authoring-form">
            <h2>{tr("教材の基本情報", "Course information")}</h2>
            <label>{tr("教材名", "Course name")}<input disabled={!workspace.editable} value={workspace.title} onChange={(event) => markDirty({ ...workspace, title: event.target.value })} /></label>
            <details className="authoring-internals">
              <summary>{tr("言語の詳細", "Language details")}</summary>
              <label>{tr("言語", "Language")}
                <select disabled={!workspace.editable} value={COMMON_LANGUAGES.find(([code]) => workspace.language.toLowerCase().split("-")[0] === code)?.[0] ?? "other"} onChange={(event) => { const value = event.target.value; if (value !== "other") markDirty({ ...workspace, language: value }); }}>
                  {COMMON_LANGUAGES.map(([code, name]) => <option key={code} value={code}>{name}</option>)}<option value="other">{tr("その他…", "Other…")}</option>
                </select>
                {!COMMON_LANGUAGES.some(([code]) => workspace.language.toLowerCase().split("-")[0] === code) || !COMMON_LANGUAGES.some(([code]) => code === workspace.language) ? <input disabled={!workspace.editable} aria-label={tr("言語コード", "Language code")} value={workspace.language} onChange={(event) => markDirty({ ...workspace, language: event.target.value })} /> : null}
              </label>
            </details>

            <section className="authoring-entity-group" aria-labelledby="concepts-heading">
              <div className="entity-group-head"><h2 id="concepts-heading">{tr("学ぶテーマ", "Topics")}</h2><button type="button" onClick={addConcept} disabled={!workspace.editable}><Plus size={15} aria-hidden="true" />{tr("テーマを追加", "Add topic")}</button></div>
              {workspace.concepts.map((item, index) => <article className="authoring-entity" key={token(item.key)}>
                <div className="entity-title"><h3>{localizedEntityName("concept", index, english)}</h3><button type="button" aria-label={tr("テーマを削除", "Delete topic")} onClick={() => markDirty({ ...workspace, concepts: workspace.concepts.filter((candidate) => !sameKey(candidate.key, item.key)) })} disabled={!workspace.editable}><Trash2 size={16} aria-hidden="true" /></button></div>
                <label>{tr("テーマ名", "Topic name")}<input disabled={!workspace.editable} value={item.title} onChange={(event) => markDirty({ ...workspace, concepts: workspace.concepts.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, title: event.target.value } : candidate) })} /></label>
                <fieldset><legend>{tr("前提となるテーマ", "Prerequisite topics")}</legend>{workspace.concepts.filter((candidate) => !sameKey(candidate.key, item.key)).map((candidate) => <label className="checkbox-row" key={token(candidate.key)}><input type="checkbox" disabled={!workspace.editable} checked={selected(item.requires, candidate.key)} onChange={(event) => markDirty({ ...workspace, concepts: workspace.concepts.map((target) => sameKey(target.key, item.key) ? { ...target, requires: updateMany(target.requires, candidate.key, event.target.checked) } : target) })} />{candidate.title || conceptName(candidate.key)}</label>)}</fieldset>
              </article>)}
            </section>

            <section className="authoring-entity-group" aria-labelledby="objectives-heading">
              <div className="entity-group-head"><h2 id="objectives-heading">{tr("できるようになること", "Learning goals")}</h2><button type="button" onClick={addObjective} disabled={!workspace.editable || workspace.concepts.length === 0}><Plus size={15} aria-hidden="true" />{tr("学習目標を追加", "Add learning goal")}</button></div>
              {workspace.objectives.map((item, index) => <article className="authoring-entity" key={token(item.key)}>
                <div className="entity-title"><h3>{localizedEntityName("objective", index, english)}</h3><button type="button" aria-label={tr("学習目標を削除", "Delete learning goal")} onClick={() => markDirty({ ...workspace, objectives: workspace.objectives.filter((candidate) => !sameKey(candidate.key, item.key)) })} disabled={!workspace.editable}><Trash2 size={16} aria-hidden="true" /></button></div>
                <label>{tr("できるようになること", "What learners can do")}<input disabled={!workspace.editable} value={item.description} onChange={(event) => markDirty({ ...workspace, objectives: workspace.objectives.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, description: event.target.value } : candidate) })} /></label>
                <label>{tr("関連するテーマ", "Topic")}<select disabled={!workspace.editable} value={token(item.concept)} onChange={(event) => markDirty({ ...workspace, objectives: workspace.objectives.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, concept: fromToken(event.target.value) } : candidate) })}>{workspace.concepts.map((concept) => <option key={token(concept.key)} value={token(concept.key)}>{concept.title || conceptName(concept.key)}</option>)}</select></label>
              </article>)}
            </section>

            <section className="authoring-entity-group" aria-labelledby="resources-heading">
              <div className="entity-group-head"><h2 id="resources-heading">{tr("教材コンテンツ", "Learning content")}</h2><button type="button" onClick={addResource} disabled={!workspace.editable || workspace.objectives.length === 0}><Plus size={15} aria-hidden="true" />{tr("教材コンテンツを追加", "Add content")}</button></div>
              {workspace.resources.map((item, index) => <article className="authoring-entity" key={token(item.key)}>
                <div className="entity-title"><h3>{localizedEntityName("resource", index, english)}</h3><button type="button" aria-label={tr("教材コンテンツを削除", "Delete content")} onClick={() => markDirty({ ...workspace, resources: workspace.resources.filter((candidate) => !sameKey(candidate.key, item.key)) })} disabled={!workspace.editable}><Trash2 size={16} aria-hidden="true" /></button></div>
                <label>{tr("教材タイトル", "Content title")}<input disabled={!workspace.editable} value={item.title} onChange={(event) => markDirty({ ...workspace, resources: workspace.resources.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, title: event.target.value } : candidate) })} /></label>
                <label>{tr("本文（Markdown）", "Content (Markdown)")}<textarea disabled={!workspace.editable} rows={12} value={item.markdown} onChange={(event) => markDirty({ ...workspace, resources: workspace.resources.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, markdown: event.target.value } : candidate) })} /></label>
                <fieldset><legend>{tr("この教材で学ぶこと", "Learning goals covered")}</legend>{workspace.objectives.map((objective) => <label className="checkbox-row" key={token(objective.key)}><input type="checkbox" disabled={!workspace.editable} checked={selected(item.teaches, objective.key)} onChange={(event) => markDirty({ ...workspace, resources: workspace.resources.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, teaches: updateMany(candidate.teaches, objective.key, event.target.checked) } : candidate) })} />{objective.description || objectiveName(objective.key)}</label>)}</fieldset>
              </article>)}
            </section>

            <section className="authoring-entity-group" aria-labelledby="curricula-heading">
              <div className="entity-group-head"><h2 id="curricula-heading">{tr("学ぶ順序", "Learning order")}</h2><button type="button" onClick={addCurriculum} disabled={!workspace.editable || workspace.objectives.length === 0}><Plus size={15} aria-hidden="true" />{tr("学ぶ順序を追加", "Add learning order")}</button></div>
              {workspace.curricula.map((item, index) => <article className="authoring-entity" key={token(item.key)}>
                <div className="entity-title"><h3>{localizedEntityName("curriculum", index, english)}</h3><button type="button" aria-label={tr("学ぶ順序を削除", "Delete learning order")} onClick={() => markDirty({ ...workspace, curricula: workspace.curricula.filter((candidate) => !sameKey(candidate.key, item.key)) })} disabled={!workspace.editable}><Trash2 size={16} aria-hidden="true" /></button></div>
                <label>{tr("順序の名前", "Order name")}<input disabled={!workspace.editable} value={item.title} onChange={(event) => markDirty({ ...workspace, curricula: workspace.curricula.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, title: event.target.value } : candidate) })} /></label>
                <fieldset><legend>{tr("含める学習目標", "Include learning goals")}</legend>{workspace.objectives.map((objective) => <label className="checkbox-row" key={token(objective.key)}><input type="checkbox" disabled={!workspace.editable} checked={selected(item.objectives, objective.key)} onChange={(event) => markDirty({ ...workspace, curricula: workspace.curricula.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, objectives: updateMany(candidate.objectives, objective.key, event.target.checked) } : candidate) })} />{objective.description || objectiveName(objective.key)}</label>)}</fieldset>
                <ol className="curriculum-order">{item.objectives.map((objectiveKey, position) => <li key={token(objectiveKey)}><span>{objectiveName(objectiveKey)}</span><button type="button" aria-label={tr("上へ", "Move up")} disabled={!workspace.editable || position === 0} onClick={() => { const reordered = [...item.objectives]; [reordered[position - 1], reordered[position]] = [reordered[position]!, reordered[position - 1]!]; markDirty({ ...workspace, curricula: workspace.curricula.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, objectives: reordered } : candidate) }); }}><ArrowUp size={15} aria-hidden="true" /></button><button type="button" aria-label={tr("下へ", "Move down")} disabled={!workspace.editable || position === item.objectives.length - 1} onClick={() => { const reordered = [...item.objectives]; [reordered[position], reordered[position + 1]] = [reordered[position + 1]!, reordered[position]!]; markDirty({ ...workspace, curricula: workspace.curricula.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, objectives: reordered } : candidate) }); }}><ArrowDown size={15} aria-hidden="true" /></button></li>)}</ol>
              </article>)}
            </section>

            <section className="authoring-entity-group" aria-labelledby="assessments-heading">
              <div className="entity-group-head"><h2 id="assessments-heading">{tr("理解を確かめる問題", "Questions")}</h2><div><button type="button" onClick={() => addAssessment("single_select")} disabled={!workspace.editable || workspace.objectives.length === 0}><Plus size={15} aria-hidden="true" />{tr("選択問題", "Multiple choice")}</button><button type="button" onClick={() => addAssessment("boolean")} disabled={!workspace.editable || workspace.objectives.length === 0}><Plus size={15} aria-hidden="true" />{tr("○×問題", "True / false")}</button></div></div>
              {workspace.assessments.map((item, index) => {
                const response = item.response;
                return <article className="authoring-entity" key={token(item.key)}>
                <div className="entity-title"><h3>{localizedEntityName("assessment", index, english)}</h3><button type="button" aria-label={tr("問題を削除", "Delete question")} onClick={() => markDirty({ ...workspace, assessments: workspace.assessments.filter((candidate) => !sameKey(candidate.key, item.key)) })} disabled={!workspace.editable}><Trash2 size={16} aria-hidden="true" /></button></div>
                <label>{tr("問題の形式", "Question type")}<select disabled={!workspace.editable} value={response.type} onChange={(event) => { const nextResponse: AssessmentResponseDraft = event.target.value === "boolean" ? { type: "boolean", answer: true } : (() => { const options = [{ key: newKey(), text: "" }, { key: newKey(), text: "" }]; return { type: "single_select", options, answer: options[0]!.key }; })(); markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, response: nextResponse } : candidate) }); }}><option value="single_select">{tr("選択問題", "Multiple choice")}</option><option value="boolean">{tr("○×問題", "True / false")}</option></select></label>
                <label>{tr("問題文", "Question")}<textarea disabled={!workspace.editable} rows={3} value={item.stimulus} onChange={(event) => markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, stimulus: event.target.value } : candidate) })} /></label>
                {response.type === "single_select" ? <>
                  <fieldset><legend>{tr("選択肢", "Answer options")}</legend>{response.options.map((option, optionIndex) => <div className="assessment-option-edit" key={token(option.key)}><label>{tr(`選択肢 ${optionIndex + 1}`, `Option ${optionIndex + 1}`)}<input disabled={!workspace.editable} value={option.text} onChange={(event) => { const nextResponse: AssessmentResponseDraft = { ...response, options: response.options.map((candidate) => sameKey(candidate.key, option.key) ? { ...candidate, text: event.target.value } : candidate) }; markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, response: nextResponse } : candidate) }); }} /></label><button type="button" aria-label={tr("選択肢を削除", "Delete option")} disabled={!workspace.editable || response.options.length <= 2} onClick={() => { const options = response.options.filter((candidate) => !sameKey(candidate.key, option.key)); const nextResponse: AssessmentResponseDraft = { ...response, options, answer: sameKey(response.answer, option.key) ? options[0]!.key : response.answer }; markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, response: nextResponse } : candidate) }); }}><Trash2 size={15} aria-hidden="true" /></button></div>)}<button type="button" disabled={!workspace.editable} onClick={() => { const nextResponse: AssessmentResponseDraft = { ...response, options: [...response.options, { key: newKey(), text: "" }] }; markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, response: nextResponse } : candidate) }); }}><Plus size={15} aria-hidden="true" />{tr("選択肢を追加", "Add option")}</button></fieldset>
                  <label>{tr("正解", "Correct answer")}<select disabled={!workspace.editable} value={token(response.answer)} onChange={(event) => { const nextResponse: AssessmentResponseDraft = { ...response, answer: fromToken(event.target.value) }; markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, response: nextResponse } : candidate) }); }}>{response.options.map((option, optionIndex) => <option key={token(option.key)} value={token(option.key)}>{tr(`選択肢 ${optionIndex + 1}`, `Option ${optionIndex + 1}`)} — {option.text}</option>)}</select></label>
                </> : <label>{tr("正解", "Correct answer")}<select disabled={!workspace.editable} value={String(response.answer)} onChange={(event) => { const nextResponse: AssessmentResponseDraft = { type: "boolean", answer: event.target.value === "true" }; markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, response: nextResponse } : candidate) }); }}><option value="true">{tr("正しい", "True")}</option><option value="false">{tr("誤り", "False")}</option></select></label>}
                <label>{tr("解説", "Explanation")}<textarea disabled={!workspace.editable} rows={3} value={item.feedback} onChange={(event) => markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, feedback: event.target.value } : candidate) })} /></label>
                <fieldset><legend>{tr("確かめる学習目標", "Learning goals measured")}</legend>{workspace.objectives.map((objective) => <label className="checkbox-row" key={token(objective.key)}><input type="checkbox" disabled={!workspace.editable} checked={selected(item.measures, objective.key)} onChange={(event) => markDirty({ ...workspace, assessments: workspace.assessments.map((candidate) => sameKey(candidate.key, item.key) ? { ...candidate, measures: updateMany(candidate.measures, objective.key, event.target.checked) } : candidate) })} />{objective.description || objectiveName(objective.key)}</label>)}</fieldset>
                </article>;
              })}
            </section>
          </section>
        </>
      ) : null}
      {!workspace && !creating ? <p>{tr("新しい教材を作るか、編集用フォルダーを開いてください。", "Create material or open an editing folder.")}</p> : null}
      {review ? <AuthoringReview references={review.references} diagnostics={review.diagnostics} /> : null}
    </section>
  );
}
