import { createContext, useContext } from "react";

export type UiLanguage = "ja" | "en";
export const UiLanguageContext = createContext<UiLanguage>("ja");
export const useUiLanguage = (): UiLanguage => useContext(UiLanguageContext);
export const localize = (language: UiLanguage, ja: string, en: string): string => language === "ja" ? ja : en;

const messages = {
  ja: { skip: "本文へ移動", library: "ライブラリ", tableOfContents: "目次を表示", choosePackage: "教材を選ぶ", progress: "進捗", history: "履歴", local: "このデバイスに保存", fontSize: "文字サイズ", executionDetails: "実行環境の詳細", close: "閉じる", language: "English" },
  en: { skip: "Skip to content", library: "Library", tableOfContents: "Show contents", choosePackage: "Choose a package", progress: "Progress", history: "History", local: "Stored on this device", fontSize: "Text size", executionDetails: "Runtime details", close: "Close", language: "日本語" },
} as const;

export function uiText(language: UiLanguage, key: keyof typeof messages.ja): string {
  return messages[language][key];
}
