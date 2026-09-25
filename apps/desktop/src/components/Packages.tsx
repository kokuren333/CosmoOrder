import { useState } from "react";
import type { ErrorView, InstalledPackageRef, PackageView } from "../types.ts";
import { ArrowRight, BookOpen, Download, RefreshCw, Trash2 } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

export function PackageList({
  packages,
  selected,
  onSelect,
  onRefresh,
  busy,
  loadState,
  loadError,
  onCreate,
  onOpenSource,
  onImport,
  onRemove,
}: {
  packages: PackageView[];
  selected: InstalledPackageRef | null;
  onSelect: (reference: InstalledPackageRef) => void;
  onRefresh: () => void;
  busy: boolean;
  loadState: "loading" | "ready" | "error";
  loadError: ErrorView[];
  onCreate: () => void;
  onOpenSource: () => void;
  onImport: () => void;
  onRemove: (reference: InstalledPackageRef) => void;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const [showAddChoices, setShowAddChoices] = useState(false);
  const addButton = <button className="primary" onClick={() => setShowAddChoices(true)}><BookOpen size={16} aria-hidden="true" />{tr("教材を追加", "Add learning material")}</button>;
  return (
    <section className="library" aria-labelledby="packages-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">{tr("学習ライブラリ", "YOUR LIBRARY")}</p>
          <h1 id="packages-heading">{tr("ライブラリ", "Library")}</h1>
          <p className="lede">{tr("教材の内容と学習目標を確認できます。", "Browse courses, their content, and learning goals.")}</p>
        </div>
        <button className="icon-button" onClick={onRefresh} disabled={busy}>
          <RefreshCw size={16} aria-hidden="true" />
          {tr("再読み込み", "Refresh")}
        </button>
        {addButton}
      </div>
      {showAddChoices ? (
        <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowAddChoices(false); }}>
          <section className="card add-package-choices" role="dialog" aria-modal="true" aria-labelledby="add-package-heading">
            <h2 id="add-package-heading">{tr("教材を追加", "Add learning material")}</h2>
            <p>{tr("新しく教材を作るか、編集用フォルダーを開きます。", "Create new material or open an editing folder.")}</p>
            <div className="authoring-actions">
              <button className="primary" onClick={() => { setShowAddChoices(false); onCreate(); }}>{tr("新しい教材を作成", "Create new material")}</button>
              <button onClick={() => { setShowAddChoices(false); onOpenSource(); }}>{tr("編集用教材フォルダーを開く", "Open an editing folder")}</button>
              <button onClick={() => { setShowAddChoices(false); onImport(); }}><Download size={15} aria-hidden="true" />{tr("CosmoOrder教材ファイルを追加", "Import a CosmoOrder course file")}</button>
              <button onClick={() => setShowAddChoices(false)}>{tr("キャンセル", "Cancel")}</button>
            </div>
          </section>
        </div>
      ) : null}
      {loadState === "loading" ? (
        <div className="empty card" role="status" aria-live="polite">
          <h2>{tr("ライブラリを読み込み中…", "Loading your library…")}</h2>
        </div>
      ) : loadState === "error" ? (
        <div className="empty card" role="alert">
          <h2>{tr("ライブラリを読み込めませんでした", "Could not load your library")}</h2>
          <ul className="diagnostics">{loadError.map((item, index) => <li key={`${item.code}-${index}`}><strong>{item.code}</strong> {item.message}</li>)}</ul>
          <button className="primary" onClick={onRefresh} disabled={busy}>{tr("再試行", "Retry")}</button>
        </div>
      ) : packages.length === 0 ? (
        <div className="empty card">
          <span className="empty-icon" aria-hidden="true"><BookOpen size={23} /></span>
          <h2>{tr("教材がありません", "No packages installed")}</h2>
          <p>{tr("教材を作成するか、編集用フォルダーを開いてライブラリに追加できます。", "Create material or open an editing folder to add it to your Library.")}</p>
          {addButton}
          <DeveloperDetails label={tr("教材の追加方法", "Add a package")}>
            <p>
              {tr("CLIから ", "Use the CLI to ")}<code>osmium install</code>{tr(" で追加してください。", " a package.")}
            </p>
          </DeveloperDetails>
        </div>
      ) : (
        <ul className="package-list">
          {packages.map((item, index) => (
            <li
              className="library-card"
              key={`${item.package_id}@${item.package_version}`}
            >
              <div className="book-cover" aria-hidden="true">
                <BookOpen size={30} aria-hidden="true" />
                <span>{tr("学習教材", "LEARNING COLLECTION")}</span>
                <strong>{String(index + 1).padStart(2, "0")}</strong>
              </div>
              <div className="library-card-body">
                <p className="meta">{tr("バージョン", "Version")} {item.package_version}</p>
                <h2>{item.title}</h2>
                {item.integrity_error ? <p className="error" role="alert">{tr("この教材ファイルは読み込めません。ライブラリから削除して再追加してください。", "This course file cannot be opened. Remove it from your Library and import it again.")}</p> : null}
                <dl className="package-counts" aria-label={tr(`${item.title}の構成`, `Contents of ${item.title}`)}>
                  {([
                    ["concepts", "テーマ", "Concepts"],
                    ["objectives", "学習目標", "Objectives"],
                    ["resources", "教材", "Resources"],
                    ["assessments", "問題", "Assessments"],
                  ] as const).map(([key, ja, en]) => (
                    <div key={key}><dt>{tr(ja, en)}</dt><dd>{item.entity_counts[key] ?? 0}</dd></div>
                  ))}
                </dl>
                <button
                  className="package primary icon-button"
                  data-package-id={item.package_id}
                  data-package-version={item.package_version}
                  disabled={busy || Boolean(item.integrity_error)}
                  onClick={() => onSelect({ package_id: item.package_id, package_version: item.package_version })}
                >
                  {selected?.package_id === item.package_id && selected.package_version === item.package_version ? tr("教材に戻る", "Return to package") : tr("教材を開く", "Open package")}
                  <ArrowRight size={16} aria-hidden="true" />
                  <span className="sr-only">: {item.title}</span>
                </button>
                <button type="button" className="ghost package-remove" disabled={busy} onClick={() => onRemove({ package_id: item.package_id, package_version: item.package_version })}>
                  <Trash2 size={15} aria-hidden="true" />{tr("ライブラリから削除", "Remove from Library")}
                </button>
                <DeveloperDetails>
                  <dl>
                    {item.integrity_error ? <><dt>Integrity diagnostic</dt><dd>{item.integrity_error}</dd></> : null}
                    <dt>Package ID</dt>
                    <dd>{item.package_id}</dd>
                    <dt>Schema</dt>
                    <dd>{item.schema_version}</dd>
                    <dt>Digest</dt>
                    <dd>{item.digest}</dd>
                    <dt>Selected version</dt>
                    <dd>{item.selected_version}</dd>
                  </dl>
                </DeveloperDetails>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
