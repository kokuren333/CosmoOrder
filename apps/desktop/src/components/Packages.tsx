import type { PackageView } from "../types.ts";
import { DeveloperDetails } from "./DeveloperDetails.tsx";

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
  return (
    <section className="library" aria-labelledby="packages-heading">
      <div className="panel-head">
        <div>
          <p className="eyebrow">YOUR LIBRARY</p>
          <h1 id="packages-heading">学びのライブラリ</h1>
          <p className="lede">手元の教材から、今日の学びを始めましょう。</p>
        </div>
        <button onClick={onRefresh} disabled={busy}>
          再読み込み
        </button>
      </div>
      {packages.length === 0 ? (
        <div className="empty card">
          <h2>教材を迎える準備ができました</h2>
          <p>インストールした教材がここに並びます。</p>
          <DeveloperDetails label="教材の追加方法">
            <p>
              CLIの <code>osmium install</code> でパッケージを追加してください。
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
                <span>OSMIUM / LEARNING</span>
                <strong>{String(index + 1).padStart(2, "0")}</strong>
                <i />
              </div>
              <div className="library-card-body">
                <p className="meta">バージョン {item.package_version}</p>
                <h2>{item.title}</h2>
                <button
                  className="package primary"
                  disabled={busy}
                  onClick={() => onSelect(item.package_id)}
                >
                  {selected === item.package_id ? "教材に戻る" : "教材を開く"}
                  <span aria-hidden="true"> →</span>
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
