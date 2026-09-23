import type { PackageView } from "../types.ts";
import { ArrowRight, BookOpen, RefreshCw } from "lucide-react";
import { DeveloperDetails } from "./DeveloperDetails.tsx";
import { localize, useUiLanguage } from "../i18n.ts";

export function PackageList({
  packages,
  selected,
  onSelect,
  onRefresh,
  busy,
}: {
  packages: PackageView[];
  selected: string | null;
  onSelect: (packageId: string) => void;
  onRefresh: () => void;
  busy: boolean;
}) {
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  return (
    <section className="library" aria-labelledby="packages-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">YOUR LIBRARY</p>
          <h1 id="packages-heading">{tr("ライブラリ", "Library")}</h1>
          <p className="lede">{tr("構造化教材Packageとその内容・学習目標を確認できます。", "Browse structured learning packages, their content, and objectives.")}</p>
        </div>
        <button className="icon-button" onClick={onRefresh} disabled={busy}>
          <RefreshCw size={16} aria-hidden="true" />
          {tr("再読み込み", "Refresh")}
        </button>
      </div>
      {packages.length === 0 ? (
        <div className="empty card">
          <span className="empty-icon" aria-hidden="true"><BookOpen size={23} /></span>
          <h2>{tr("Packageがありません", "No packages installed")}</h2>
          <p>{tr("osmium installで追加した教材がここに表示されます。", "Packages installed with osmium install will appear here.")}</p>
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
                <span>LEARNING COLLECTION</span>
                <strong>{String(index + 1).padStart(2, "0")}</strong>
              </div>
              <div className="library-card-body">
                <p className="meta">{tr("バージョン", "Version")} {item.package_version}</p>
                <h2>{item.title}</h2>
                <dl className="package-counts" aria-label={tr(`${item.title}の構成`, `Contents of ${item.title}`)}>
                  {([
                    ["concepts", "Concepts"],
                    ["objectives", "Objectives"],
                    ["resources", "Resources"],
                    ["assessments", "Assessments"],
                  ] as const).map(([key, label]) => (
                    <div key={key}><dt>{label}</dt><dd>{item.entity_counts[key] ?? 0}</dd></div>
                  ))}
                </dl>
                <button
                  className="package primary icon-button"
                  disabled={busy}
                  onClick={() => onSelect(item.package_id)}
                >
                  {selected === item.package_id ? tr("教材に戻る", "Return to package") : tr("教材を開く", "Open package")}
                  <ArrowRight size={16} aria-hidden="true" />
                  <span className="sr-only">: {item.title}</span>
                </button>
                <DeveloperDetails>
                  <dl>
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
