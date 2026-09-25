import { useState } from "react";
import { localize, useUiLanguage } from "../i18n.ts";

type Scenario = "baseline" | "preload" | "afterload";

/** Runtime-owned, allowlisted illustration; package data cannot select code or supply executable content. */
export function PressureVolumePrototype() {
  const [scenario, setScenario] = useState<Scenario>("baseline");
  const [answer, setAnswer] = useState<string | null>(null);
  const language = useUiLanguage();
  const tr = (ja: string, en: string) => localize(language, ja, en);
  const values = scenario === "preload"
    ? { edv: 245, esv: 130, peak: 82 }
    : scenario === "afterload"
      ? { edv: 205, esv: 160, peak: 48 }
      : { edv: 205, esv: 130, peak: 82 };
  const points = `70,230 ${values.edv},230 ${values.edv},${values.peak} ${values.esv},92 ${values.esv},230`;
  const correct = answer === "preload";
  return <section className="pv-prototype" aria-labelledby="pv-prototype-heading">
    <p className="eyebrow">{tr("操作して観察する · 試作", "INTERACTIVE PRACTICE · PROTOTYPE")}</p>
    <h2 id="pv-prototype-heading">{tr("圧・容積ループを比べる", "Compare pressure–volume loops")}</h2>
    <p>{tr("条件を選んで、模式図の変化を観察してください。教育用の単純化した図であり、実測値や臨床予測ではありません。", "Choose a change and observe this simplified teaching diagram. It is not measured data or a clinical prediction.")}</p>
    <p className="meta">{tr("操作欄に問題がある場合も、上の教材本文から学習を続けられます。", "If this interactive panel is unavailable, continue with the resource text above.")}</p>
    <div className="pv-controls" role="group" aria-label={tr("模式図の条件", "Diagram condition")}>
      <button type="button" aria-pressed={scenario === "baseline"} onClick={() => { setScenario("baseline"); setAnswer(null); }}>{tr("基準", "Baseline")}</button>
      <button type="button" aria-pressed={scenario === "preload"} onClick={() => { setScenario("preload"); setAnswer(null); }}>{tr("前負荷を増やす", "Increase preload")}</button>
      <button type="button" aria-pressed={scenario === "afterload"} onClick={() => { setScenario("afterload"); setAnswer(null); }}>{tr("後負荷を増やす", "Increase afterload")}</button>
    </div>
    <svg className="pv-diagram" viewBox="0 0 300 270" role="img" aria-label={tr(`模式的な圧・容積ループ。拡張末期容積 ${values.edv}、収縮末期容積 ${values.esv}`, `Illustrative pressure–volume loop. End-diastolic volume ${values.edv}, end-systolic volume ${values.esv}`)}>
      <path d="M45 230H275M45 230V25" className="pv-axis" />
      <polyline points={points} className="pv-loop" />
      <text x="150" y="260">{tr("容積（模式）", "Volume (illustrative)")}</text>
      <text x="8" y="24">{tr("圧", "Pressure")}</text>
      <text x={values.edv - 20} y="248">EDV</text><text x={values.esv - 18} y="248">ESV</text>
    </svg>
    <p className="pv-observation" aria-live="polite">{scenario === "preload" ? tr("観察: ループ右端（EDV）が右へ移動しました。", "Observe: the right boundary (EDV) moved right.") : scenario === "afterload" ? tr("観察: 駆出開始側の圧が上がり、模式図のESVが右へ移動しました。", "Observe: the ejection pressure is higher and illustrative ESV moved right.") : tr("基準状態です。各条件を選んで変化を比べてください。", "Baseline. Select a change to compare.")}</p>
    <fieldset className="pv-question" disabled={answer !== null}><legend>{tr("この図でEDVが増えた条件は？", "Which condition increased EDV in this diagram?")}</legend>
      <label><input type="radio" name="pv-answer" value="preload" checked={answer === "preload"} onChange={() => setAnswer("preload")} />{tr("前負荷を増やす", "Increase preload")}</label>
      <label><input type="radio" name="pv-answer" value="afterload" checked={answer === "afterload"} onChange={() => setAnswer("afterload")} />{tr("後負荷を増やす", "Increase afterload")}</label>
    </fieldset>
    {answer !== null ? <div className={correct ? "pv-feedback correct" : "pv-feedback incorrect"} role="status"><p>{correct ? tr("正解です。基準と前負荷条件をもう一度比べてください。", "Correct. Compare the baseline and preload conditions again.") : tr("この図では前負荷条件でEDVが増えています。条件を切り替えて観察し直してください。", "In this diagram, EDV increases in the preload condition. Switch conditions and inspect again.")}</p><button type="button" className="ghost" onClick={() => { setAnswer(null); setScenario("baseline"); }}>{tr("もう一度", "Try again")}</button></div> : null}
    <p className="meta">{tr("試作の回答は学習履歴へ保存されません。Packageから任意のスクリプトは実行しません。", "Prototype responses are not stored in learning history. No Package script is executed.")}</p>
  </section>;
}
