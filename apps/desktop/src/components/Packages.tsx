import type { ReactElement } from "react";
import type { PackageView } from "../types.ts";

/** Installed packages, as the library reports them. */
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
}): ReactElement {
  return (
    <section className="panel" aria-labelledby="packages-heading">
      <div className="panel-head">
        <h2 id="packages-heading">インストール済みパッケージ</h2>
        <button type="button" onClick={onRefresh} disabled={busy}>
          再読み込み
        </button>
      </div>
      {packages.length === 0 ? (
        <p className="empty">
          インストール済みパッケージがありません。CLIの <code>osmium install</code> で
          追加してください。
        </p>
      ) : (
        <ul className="package-list">
          {packages.map((item) => (
            <li key={`${item.package_id}@${item.package_version}`}>
              <button
                type="button"
                className={selected === item.package_id ? "package selected" : "package"}
                aria-current={selected === item.package_id ? "true" : undefined}
                onClick={() => onSelect(item.package_id)}
              >
                <span className="package-title">{item.title}</span>
                <span className="package-id">{item.package_id}</span>
                <span className="package-meta">
                  version {item.package_version} · schema {item.schema_version}
                </span>
                <span className="package-digest">digest {item.digest.slice(0, 16)}…</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
