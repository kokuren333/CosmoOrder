"""Create compact, original cross-domain Osmium pressure-test packages."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "examples"

domains = {
    "medicine": {
        "title": "Clinical reasoning: dehydration",
        "language": "en",
        "concepts": [("fluid", "Fluid balance", []), ("vitals", "Vital signs", ["fluid"]), ("assessment", "Clinical assessment", ["vitals"]), ("reasoning", "Integrated reasoning", ["assessment"])],
        "resources": [
            ("fluid", "Fluid balance", "Fluid balance describes intake, distribution, and loss. A deficit can reduce circulating volume.\n\n| Term | Meaning | Clinical context |\n|---|---|---|\n| Intake | Water and electrolytes received | Food and fluids consumed by mouth or enteral route |\n| Output | Measured or estimated loss | Urine, gastrointestinal losses, and other documented routes |\n\n**Dehydration** is a clinical state; a single sign does not establish its cause. Sources: [WHO public guidance](https://www.who.int/news-room/fact-sheets/detail/drinking-water). Creator: Osmium sample authors; license: CC0-1.0; original educational text."),
            ("vitals", "Vital signs", "Temperature, pulse, blood pressure, and respiratory rate provide context. Interpret trends with history and examination; measurement error and patient context matter.\n\n> A finding is evidence, not a diagnosis."),
            ("reasoning", "Case assessment", "A hypothetical adult reports two days of vomiting and dizziness on standing. Document uncertainty, assess stability, and seek qualified clinical supervision. This fictional case is for learning, not care guidance."),
        ],
        "objectives": [("fluid.define", "fluid", "Explain intake and output"), ("vitals.interpret", "vitals", "Interpret a vital-sign trend in context"), ("case.integrate", "reasoning", "Integrate fluid history, vital signs, and examination findings")],
        "assessment": ("case.check", ["case.integrate"], "In the fictional case, which is the most appropriate first reasoning step?", ["Assume a diagnosis from one symptom", "Assess stability and gather contextual findings"], "b", "Link evidence to the reasoning concepts without treating this case as clinical advice."),
        "curriculum": ["fluid.define", "vitals.interpret", "case.integrate"],
    },
    "mathematics": {
        "title": "Probability and conditional reasoning", "language": "en",
        "concepts": [("probability", "Probability", []), ("events", "Events", ["probability"]), ("conditional", "Conditional probability", ["events"]), ("bayes", "Bayes' rule", ["conditional"])],
        "resources": [
            ("probability", "Probability basics", "For events, $0 \\le P(A) \\le 1$. The complement has probability $P(A^c)=1-P(A)$.\n\nA probability model assigns mass consistently across outcomes."),
            ("conditional", "Conditional probability", "When $P(B)>0$,\n\n$$\nP(A \\mid B)=\\frac{P(A \\cap B)}{P(B)}\n$$\n\nFor a two-state vector, $\\begin{bmatrix}P(A)\\\\P(A^c)\\end{bmatrix}$, entries sum to 1. Superscripts $x^2$, subscripts $a_i$, Greek symbols $\\alpha, \\beta$, and a long identity $P(A \\cap B)=P(A \\mid B)P(B)$ are common notation."),
        ],
        "objectives": [("probability.bounds", "probability", "Check probability bounds"), ("conditional.compute", "conditional", "Compute a conditional probability from a table"), ("bayes.apply", "bayes", "Apply Bayes' rule and state its assumptions")],
        "assessment": ("bayes.check", ["bayes.apply"], "If $P(A)=0.2$, $P(B|A)=0.5$, and $P(B)=0.4$, find $P(A|B)$.", ["0.25", "0.4"], "a", "Substitute into Bayes' rule: $P(A|B)=P(B|A)P(A)/P(B)$."),
        "curriculum": ["probability.bounds", "conditional.compute", "bayes.apply"],
    },
    "language": {
        "title": "Japanese and English: asking for directions", "language": "ja",
        "concepts": [("politeness", "Polite requests", []), ("directions", "Direction phrases", ["politeness"]), ("pronunciation", "Pronunciation cues", ["directions"])],
        "resources": [
            ("politeness", "A polite request", "日本語では「駅への行き方を教えていただけますか」と尋ねられます。\n\n**Could you tell me how to get to the station?** is a polite English equivalent. *Could you* softens the request.\n\nExample: “Excuse me, could you tell me how to get to the station?” —「すみません、駅への行き方を教えていただけますか。」"),
            ("directions", "Direction phrases", "| English | 日本語 |\n|---|---|\n| Turn left | 左に曲がる |\n| Go straight | まっすぐ進む |\n| across from | ～の向かい |\n\nIPA: station /ˈsteɪʃən/, straight /streɪt/. Unicode check: café, 東京, 한글."),
        ],
        "objectives": [("request.form", "politeness", "Choose a polite request form"), ("directions.translate", "directions", "Match direction phrases across Japanese and English"), ("pronounce.station", "pronunciation", "Recognize the stressed syllable in station")],
        "assessment": ("request.check", ["request.form"], "Which is the more polite request?", ["Tell me the way.", "Could you tell me the way?"], "b", "Could you is a conventional softener in this request."),
        "curriculum": ["request.form", "directions.translate", "pronounce.station"],
    },
    "programming": {
        "title": "Reading safe program output", "language": "en",
        "concepts": [("values", "Values and expressions", []), ("errors", "Error messages", ["values"]), ("diffs", "Code review diffs", ["errors"])],
        "resources": [
            ("values", "Inspecting code as text", "The inline expression `2 + 2` evaluates in many languages, but Osmium displays package code and never executes it.\n\n```python\nmessage = \"hello\"\nprint(message)\nresult = transform(input_value, configuration, optional_context, validation_mode, required_context, audit_metadata)\n```\n\nLong lines remain horizontally scrollable."),
            ("errors", "Output and errors", "Console output is evidence about a run, not proof of correctness.\n\n```text\n$ python example.py\nhello\nTraceback (most recent call last):\n  ValueError: invalid input\n```\n\nHTML/XML source is shown as code: `<img src=x onerror=alert(1)>` and `&lt;node attr=\"x\"/&gt;`."),
            ("diffs", "Reviewing a diff", "```diff\n- timeout = 10\n+ timeout = 20\n```\n\nA diff communicates proposed text changes; it does not apply or run them."),
        ],
        "objectives": [("values.read", "values", "Identify a literal and a function call"), ("errors.read", "errors", "Locate the exception type in output"), ("diffs.review", "diffs", "Describe a changed line in a diff")],
        "assessment": ("errors.check", ["errors.read"], "Which token names the exception type?\n\n```text\nValueError: invalid input\n```", ["ValueError", "input"], "a", "The exception type precedes the message text."),
        "curriculum": ["values.read", "errors.read", "diffs.review"],
    },
}

for slug, d in domains.items():
    if slug == "medicine":
        # The medicine pressure test is a curated Japanese learning resource,
        # not a generated compact renderer fixture. Preserve it when this
        # script refreshes the other domains.
        continue
    root = ROOT / f"{slug}-pressure-test"
    (root / "entities").mkdir(parents=True, exist_ok=True)
    (root / "content").mkdir(exist_ok=True)
    manifest = {"schema_version":"0.1", "package_id":f"org.osmium.pressure/{slug}", "package_version":"0.1.0", "title":d["title"], "language":d["language"], "capabilities":{"required":[],"optional":[]}, "entities":{"concepts":"entities/concepts.json","objectives":"entities/objectives.json","curricula":"entities/curricula.json","resources":"entities/resources.json","assessments":"entities/assessments.json"},"extensions":{}}
    concepts = [{"id":i,"title":t,"requires":r} for i,t,r in d["concepts"]]
    objectives = [{"id":i,"concept":c,"description":desc} for i,c,desc in d["objectives"]]
    resources=[]
    for id,title,body in d["resources"]:
        path=f"content/{id}.md"; (root/path).write_text(f"# {title}\n\n{body}\n", encoding="utf-8")
        teaches=[o[0] for o in d["objectives"] if o[1]==id]
        resource={"id":f"{id}.lesson","type":"markdown","title":title,"path":path,"teaches":teaches,"language":d["language"],"creator":"Osmium sample authors","license":"CC0-1.0","attribution":"Original fictional pressure-test text"}
        if slug == "medicine":
            resource["source"] = "https://www.who.int/news-room/fact-sheets/detail/drinking-water"
            resource["provenance"] = {"content_origin":"original sample text", "human_review":"sample metadata only"}
        resources.append(resource)
    covered = {o[1] for o in d["objectives"] if any(o[0] in r["teaches"] for r in resources)}
    objective_concepts = {o[1] for o in d["objectives"]}
    for concept_id, concept_title, _ in d["concepts"]:
        if concept_id not in objective_concepts:
            continue
        if concept_id not in covered:
            objective_ids = [o[0] for o in d["objectives"] if o[1] == concept_id]
            rid = f"{concept_id}.lesson"
            path = f"content/{rid}.md"
            (root/path).write_text(f"# {concept_title}\n\nThis short original note introduces {concept_title.lower()} and connects it to the surrounding learning sequence.\n", encoding="utf-8")
            resources.append({"id":rid,"type":"markdown","title":concept_title,"path":path,"teaches":objective_ids,"creator":"Osmium sample authors","license":"CC0-1.0","attribution":"Original pressure-test text"})
    aid, measures, prompt, options, answer, feedback = d["assessment"]
    measures = [o[0] for o in d["objectives"]]
    assessment={"id":aid,"revision":"1","measures":measures,"stimulus":{"markdown":prompt},"response":{"type":"single_select","options":[{"id":chr(97+i),"text":text} for i,text in enumerate(options)]},"evaluation":{"type":"exact","answer":answer},"feedback":{"markdown":feedback}}
    followup={"id":aid+".review","revision":"1","measures":measures,"stimulus":{"markdown":f"Which statement best summarizes the concepts in **{d['title']}**?"},"response":{"type":"single_select","options":[{"id":"a","text":"Use evidence and definitions from the resources"},{"id":"b","text":"Infer an answer without checking the material"}]},"evaluation":{"type":"exact","answer":"a"},"feedback":{"markdown":"Return to the related resources and connect the stated concepts to the objective."}}
    entities={"concepts":concepts,"objectives":objectives,"curricula":[{"id":"core","title":"Core sequence","objectives":d["curriculum"]}],"resources":resources,"assessments":[assessment,followup]}
    for name,obj in [("osmium.json",manifest), *[(f"entities/{k}.json",v) for k,v in entities.items()]]:
        (root/name).write_text(json.dumps(obj,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
