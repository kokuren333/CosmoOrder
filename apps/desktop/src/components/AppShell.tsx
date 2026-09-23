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
import { UiLanguageContext, uiText, type UiLanguage } from "../i18n.ts";

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
  language,
  onLanguage,
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
  language: UiLanguage;
  onLanguage: (language: UiLanguage) => void;
  children: ReactNode;
}) {
  const learning = ["lesson", "reader", "assessment"].includes(section);
  const t = (key: Parameters<typeof uiText>[1]) => uiText(language, key);
  return (
    <UiLanguageContext.Provider value={language}>
    <div className="app" style={{ fontSize: `${scale}rem` }}>
      <a className="skip-link" href="#main-content">
        {t("skip")}
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
          </span>
        </button>
        <div className="header-tools">
          <span className="local-label">{t("local")}</span>
          <button type="button" onClick={() => onLanguage(language === "ja" ? "en" : "ja")} aria-label={t("language")}>{t("language")}</button>
          <details className="display-settings">
            <summary><Type size={15} aria-hidden="true" />{t("fontSize")}</summary>
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
          {t("library")}
        </button>
        <button
          className="course-nav"
          aria-label={t("tableOfContents")}
          aria-current={learning ? "page" : undefined}
          onClick={onLesson}
          disabled={busy || title === null}
        >
          <BookOpenText size={17} aria-hidden="true" />
          {title ?? t("choosePackage")}
        </button>
        <button
          aria-label={t("progress")}
          aria-current={section === "progress" ? "page" : undefined}
          onClick={onProgress}
          disabled={busy || title === null}
        >
          <ChartNoAxesColumnIncreasing size={17} aria-hidden="true" />
          {t("progress")}
        </button>
        <button
          aria-label={t("history")}
          aria-current={section === "history" ? "page" : undefined}
          onClick={onHistory}
          disabled={busy || title === null}
        >
          <History size={17} aria-hidden="true" />
          {t("history")}
        </button>
      </nav>
      <main id="main-content" tabIndex={-1}>
        {children}
      </main>
      <footer className="app-footer">
        {status !== null ? (
          <DeveloperDetails label={t("executionDetails")}>
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
    </UiLanguageContext.Provider>
  );
}
