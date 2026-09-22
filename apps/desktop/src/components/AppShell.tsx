import type { ReactNode } from "react";
import {
  BookOpenText,
  ChartNoAxesColumnIncreasing,
  History,
  Library,
  Type,
} from "lucide-react";
import type { StatusView } from "../types.ts";
import { BrandGem } from "./BrandGem.tsx";
import { DeveloperDetails } from "./DeveloperDetails.tsx";

export function AppShell({
  section,
  title,
  scale,
  busy,
  status,
  onScale,
  onLibrary,
  onLesson,
  onProgress,
  onHistory,
  children,
}: {
  section: string;
  title: string | null;
  scale: number;
  busy: boolean;
  status: StatusView | null;
  onScale: (value: number) => void;
  onLibrary: () => void;
  onLesson: () => void;
  onProgress: () => void;
  onHistory: () => void;
  children: ReactNode;
}) {
  const learning = ["lesson", "reader", "assessment"].includes(section);
  return (
    <div className="app" style={{ fontSize: `${scale}rem` }}>
      <a className="skip-link" href="#main-content">
        本文へ移動
      </a>
      <header className="app-header">
        <button
          className="brand ghost"
          onClick={onLibrary}
          disabled={busy}
          aria-label="Osmium ライブラリへ"
        >
          <span className="brand-mark">
            <BrandGem />
          </span>
          <span className="brand-copy">
            <span>Osmium</span>
            <small>学びを、もっと身近に。</small>
          </span>
        </button>
        <div className="header-tools">
          <span className="local-label">このデバイスで学ぶ</span>
          <details className="display-settings">
            <summary><Type size={15} aria-hidden="true" />文字サイズ</summary>
            <div className="row">
              {[1, 1.5, 2].map((value) => (
                <button
                  key={value}
                  aria-pressed={scale === value}
                  onClick={() => onScale(value)}
                >
                  文字 {value * 100}%
                </button>
              ))}
            </div>
          </details>
        </div>
      </header>
      <nav className="app-nav" aria-label="メインナビゲーション">
        <button
          aria-current={section === "packages" ? "page" : undefined}
          onClick={onLibrary}
          disabled={busy}
        >
          <Library size={17} aria-hidden="true" />
          ライブラリ
        </button>
        <button
          className="course-nav"
          aria-label="目次を表示"
          aria-current={learning ? "page" : undefined}
          onClick={onLesson}
          disabled={busy || title === null}
        >
          <BookOpenText size={17} aria-hidden="true" />
          {title ?? "教材を選ぶ"}
        </button>
        <button
          aria-label="進捗を表示"
          aria-current={section === "progress" ? "page" : undefined}
          onClick={onProgress}
          disabled={busy || title === null}
        >
          <ChartNoAxesColumnIncreasing size={17} aria-hidden="true" />
          進捗
        </button>
        <button
          aria-label="履歴を表示"
          aria-current={section === "history" ? "page" : undefined}
          onClick={onHistory}
          disabled={busy || title === null}
        >
          <History size={17} aria-hidden="true" />
          履歴
        </button>
      </nav>
      <main id="main-content" tabIndex={-1}>
        {children}
      </main>
      <footer className="app-footer">
        <p>学びの記録は、このデバイスに保存されます。</p>
        {status !== null ? (
          <DeveloperDetails label="実行環境の詳細">
            <dl>
              <dt>OSMIUM_HOME</dt>
              <dd>{status.home}</dd>
              <dt>Database</dt>
              <dd>{status.database}</dd>
              <dt>State version</dt>
              <dd>{status.state_version}</dd>
            </dl>
          </DeveloperDetails>
        ) : null}
      </footer>
    </div>
  );
}
