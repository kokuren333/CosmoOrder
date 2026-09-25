// End-to-end acceptance for the Osmium desktop application.
//
// This script drives the packaged application itself: it installs the Golden
// package with the CLI, starts the real Windows executable, and then talks to
// the webview over the Chrome DevTools Protocol to read the DOM and click the
// real controls. It asserts the whole v1 vertical slice, including that the
// webview performs no network request other than the application's own local
// assets and its IPC channel, and that progress survives a full restart.
//
// Usage:
//   node e2e/desktop-e2e.mjs --home <data-dir> --app <path-to-osmium-desktop.exe>
//                             [--repo <repo-root>] [--port 9333]
//
// Exit code 0 means every step passed.

import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const defaultRepo = resolve(here, "..", "..", "..");

function parseArgs(argv) {
  const options = { port: 9333, repo: defaultRepo, osmium: null, authoringOnly: false };
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (key === "--authoring-only") {
      options.authoringOnly = true;
      continue;
    }
    const value = argv[index + 1];
    if (key === "--home") options.home = value;
    else if (key === "--app") options.app = value;
    else if (key === "--repo") options.repo = value;
    else if (key === "--osmium") options.osmium = value;
    else if (key === "--port") options.port = Number(value);
    else throw new Error(`unknown argument: ${key}`);
    index += 1;
  }
  if (options.home === undefined) throw new Error("--home is required");
  if (options.app === undefined) throw new Error("--app is required");
  options.osmium ??= join(options.repo, "target", "debug", "osmium.exe");
  return options;
}

const options = parseArgs(process.argv.slice(2));
const home = resolve(options.home);
const appPath = resolve(options.app);
const tempRoot = resolve(tmpdir());
const homeFromTemp = relative(tempRoot, home);
const debugUrl = `http://127.0.0.1:${options.port}`;
const PACKAGE_ID = "org.example/arithmetic";

const steps = [];
const runningApps = new Set();
function pass(name, detail = "") {
  steps.push(name);
  console.log(`PASS ${name}${detail === "" ? "" : `: ${detail}`}`);
}
function fail(name, detail) {
  console.error(`FAIL ${name}: ${detail}`);
  throw new Error(`${name}: ${detail}`);
}
function assert(condition, name, detail) {
  if (condition) pass(name, detail);
  else fail(name, detail);
}

function cli(args, { expectSuccess = true } = {}) {
  // The release CLI is prebuilt by the caller: this script must not compile.
  const result = spawnSync(options.osmium, ["--home", home, ...args], {
    cwd: options.repo,
    encoding: "utf8",
  });
  if (expectSuccess && result.status !== 0) {
    fail(`cli ${args.join(" ")}`, result.stderr || `exit ${result.status}`);
  }
  let payload = null;
  try {
    payload = JSON.parse(result.stdout.trim());
  } catch {
    payload = null;
  }
  return {
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
    payload,
  };
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

class App {
  constructor() {
    runningApps.add(this);
    this.child = null;
    this.socket = null;
    this.nextId = 1;
    this.pending = new Map();
    this.requests = [];
    this.consoleErrors = [];
    this.stderr = "";
    this.exited = undefined;
  }

  async start() {
    this.child = spawn(appPath, [], {
      env: {
        ...process.env,
        OSMIUM_HOME: home,
        OSMIUM_DEBUG_PORT: String(options.port),
        WEBVIEW2_USER_DATA_FOLDER: join(home, "webview-e2e-profile"),
      },
      stdio: ["ignore", "pipe", "pipe"],
    });
    this.child.stderr.on("data", (chunk) => {
      this.stderr += chunk.toString();
    });
    this.child.on("exit", (code) => {
      this.exited = code;
    });
    const target = await this.waitForTarget();
    await this.connect(target);
    await this.waitForSelector("h1", "the application heading");
  }

  async waitForTarget() {
    for (let attempt = 0; attempt < 80; attempt += 1) {
      if (this.exited !== undefined) {
        throw new Error(
          `the application exited with code ${this.exited}: ${this.stderr}`,
        );
      }
      try {
        const response = await fetch(`${debugUrl}/json/list`);
        const list = await response.json();
        const page = list.find((entry) => entry.type === "page");
        if (page !== undefined) return page;
      } catch {
        // The webview is not up yet.
      }
      await sleep(500);
    }
    throw new Error(
      `the application did not expose ${debugUrl}${this.stderr === "" ? "" : `: ${this.stderr}`}`,
    );
  }

  async connect(target) {
    this.socket = new WebSocket(target.webSocketDebuggerUrl);
    this.socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      if (message.id !== undefined && this.pending.has(message.id)) {
        this.pending.get(message.id)(message);
        this.pending.delete(message.id);
      } else if (message.method === "Network.requestWillBeSent") {
        this.requests.push(message.params.request.url);
      } else if (message.method === "Runtime.exceptionThrown") {
        this.consoleErrors.push(
          JSON.stringify(message.params.exceptionDetails),
        );
      }
    });
    await new Promise((done, reject) => {
      this.socket.addEventListener("open", done);
      this.socket.addEventListener("error", reject);
    });
    await this.send("Runtime.enable");
    await this.send("Network.enable");
  }

  send(method, params = {}) {
    const id = this.nextId;
    this.nextId += 1;
    return new Promise((done) => {
      this.pending.set(id, done);
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  async evaluate(expression) {
    const response = await this.send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (response.result?.exceptionDetails !== undefined) {
      throw new Error(
        `page exception: ${JSON.stringify(response.result.exceptionDetails)}`,
      );
    }
    return response.result?.result?.value;
  }

  text() {
    return this.evaluate("document.body.innerText");
  }

  async waitFor(predicate, description, attempts = 80) {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      if ((await this.evaluate(`(${predicate})()`)) === true) return;
      await sleep(250);
    }
    throw new Error(
      `timed out waiting for ${description}. Body was:\n${await this.text()}`,
    );
  }

  waitForSelector(selector, description) {
    return this.waitFor(
      `() => document.querySelector(${JSON.stringify(selector)}) !== null`,
      description,
    );
  }

  /** Click the first element matching a CSS selector. */
  async click(selector, description) {
    await this.waitForSelector(selector, description);
    await this.waitFor(
      `() => !document.querySelector(${JSON.stringify(selector)}).matches(":disabled")`,
      `${description} is ready`,
    );
    const clicked = await this.evaluate(`(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (element === null) return false;
      element.click();
      return true;
    })()`);
    if (clicked !== true)
      fail(`click ${description}`, `no element for ${selector}`);
  }

  /** Click the first button whose exact label matches. */
  async clickText(label, description) {
    await this.waitFor(
      `() => [...document.querySelectorAll("button")].some((candidate) => candidate.textContent.trim() === ${JSON.stringify(label)} && !candidate.disabled)`,
      `${description} is ready`,
    );
    const clicked = await this.evaluate(`(() => {
      const button = [...document.querySelectorAll("button")].find(
        (candidate) => candidate.textContent.trim() === ${JSON.stringify(label)},
      );
      if (button === undefined) return false;
      button.click();
      return true;
    })()`);
    if (clicked !== true) {
      fail(`click ${description}`, `no button labelled ${label}`);
    }
  }

  /** Click the first button whose rendered label starts with the given text. */
  async clickTextStartingWith(label, description) {
    await this.waitFor(
      `() => [...document.querySelectorAll("button")].some((candidate) => candidate.textContent.trim().startsWith(${JSON.stringify(label)}) && !candidate.disabled)`,
      `${description} is ready`,
    );
    const clicked = await this.evaluate(`(() => {
      const button = [...document.querySelectorAll("button")].find(
        (candidate) => candidate.textContent.trim().startsWith(${JSON.stringify(label)}),
      );
      if (button === undefined) return false;
      button.click();
      return true;
    })()`);
    if (clicked !== true) {
      fail(`click ${description}`, `no button starting with ${label}`);
    }
  }

  async setField(selector, value) {
    const changed = await this.evaluate(`(() => {
      const field = document.querySelector(${JSON.stringify(selector)});
      if (field === null) return false;
      const descriptor = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), 'value');
      descriptor?.set?.call(field, ${JSON.stringify(value)});
      field.dispatchEvent(new Event('input', { bubbles: true }));
      field.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
    if (changed !== true) fail("set field", `no field for ${selector}`);
  }

  async stop() {
    runningApps.delete(this);
    try {
      this.socket?.close();
    } catch {
      // The socket may already be gone.
    }
    if (this.child !== null) {
      // The webview keeps a child process tree, so kill the whole tree.
      spawnSync("taskkill", ["/PID", String(this.child.pid), "/T", "/F"], {
        stdio: "ignore",
      });
      this.child = null;
    }
    for (let attempt = 0; attempt < 60 && this.exited === undefined; attempt += 1) {
      await sleep(100);
    }
    if (this.exited === undefined) throw new Error("the application process did not exit after taskkill");
    for (let attempt = 0; attempt < 60; attempt += 1) {
      await sleep(250);
      try {
        await fetch(`${debugUrl}/json/version`, {
          signal: AbortSignal.timeout(500),
        });
      } catch {
        // WebView2 releases its user-data profile asynchronously after the
        // listener closes. Let that cleanup finish before starting a new app.
        await sleep(1000);
        return;
      }
    }
    throw new Error("the application did not shut down");
  }
}

async function captureScreenshot(app, name) {
  const directory = join(home, "screenshots");
  mkdirSync(directory, { recursive: true });
  const captured = await app.send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(directory, `${name}.png`), Buffer.from(captured.result.data, "base64"));
}

const GOLDEN_MARKER = "全部で2個になります";
const GOLDEN_STIMULUS = "1 + 1";
const GOLDEN_FEEDBACK = "1に1を足すと2です";
const GOLDEN_OBJECTIVE = "小さな整数の足し算ができる";

// Exercise every page at the supported text sizes and both system themes.
// Captures are local QA artifacts; no app behavior or package content is injected.
async function checkPresentation(app, name) {
  const directory = join(home, "screenshots");
  mkdirSync(directory, { recursive: true });
  for (const viewportWidth of [390, 1280]) {
    await app.send("Emulation.setDeviceMetricsOverride", {
      width: viewportWidth,
      height: 900,
      deviceScaleFactor: 1,
      mobile: false,
    });
  for (const theme of ["light", "dark"]) {
    await app.send("Emulation.setEmulatedMedia", {
      features: [
        { name: "prefers-color-scheme", value: theme },
        { name: "prefers-reduced-motion", value: "reduce" },
      ],
    });
    await app.evaluate(
      'document.querySelector(".display-settings").open = true',
    );
    for (const scale of [100, 150, 200]) {
      await app.clickText(`${scale}%`, "text scale");
      await app.waitFor(
        `() => document.querySelector(".app").style.fontSize === "${scale / 100}rem"`,
        "updated scale",
      );
      assert(
        await app.evaluate(
          "document.documentElement.scrollWidth <= window.innerWidth + 1",
        ),
        `${name}: ${theme} ${scale}% fits the window`,
      );
      if (
        (theme === "light" && scale === 100) ||
        (theme === "dark" && scale === 200)
      ) {
        await app.evaluate(
          'document.querySelector(".display-settings").open = false',
        );
        const metrics = await app.send("Page.getLayoutMetrics");
        const size = metrics.result.cssContentSize;
        const screenshot = await app.send("Page.captureScreenshot", {
          captureBeyondViewport: true,
          clip: {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
            scale: 1,
          },
        });
        writeFileSync(
          join(directory, `${name}-${viewportWidth}-${theme}-${scale}.png`),
          Buffer.from(screenshot.result.data, "base64"),
        );
        await app.evaluate(
          'document.querySelector(".display-settings").open = true',
        );
      }
    }
  }
  }
  await app.send("Emulation.clearDeviceMetricsOverride");
  await app.clickText("100%", "restore text size");
  await app.evaluate(
    'document.querySelector(".display-settings").open = false',
  );
  await app.send("Emulation.setEmulatedMedia", {
    features: [{ name: "prefers-color-scheme", value: "light" }],
  });
}

async function main() {
  console.log("# Osmium desktop end-to-end acceptance");
  console.log(`data root: ${home}`);
  console.log(`application: ${appPath}`);

  if (!existsSync(appPath))
    fail("application binary", `${appPath} does not exist`);
  if (homeFromTemp === "" || homeFromTemp.startsWith("..") || isAbsolute(homeFromTemp)) {
    fail("isolated test data", `--home must be a dedicated directory under the OS temp root (${tempRoot})`);
  }
  if (existsSync(home) && readdirSync(home).length !== 0) {
    fail("isolated test data", "--home must be a new or empty directory");
  }
  mkdirSync(home, { recursive: true });

  // 1. Golden package: validate, lint, build and install, all offline.
  const source = join(options.repo, "examples", "arithmetic");
  const archive = join(home, "arithmetic.osmium");

  const validated = cli(["validate", source, "--json"]);
  assert(
    validated.payload?.ok === true,
    "CLI validate accepts the Golden source",
    JSON.stringify(validated.payload?.data ?? {}),
  );

  const linted = cli(["lint", source, "--json"]);
  assert(
    linted.payload?.ok === true,
    "CLI lint accepts the Golden source",
    `findings ${linted.payload?.data?.finding_count}`,
  );

  const built = cli(["build", source, "--output", archive]);
  assert(
    built.payload?.ok === true,
    "CLI build produces a portable archive",
    built.payload?.data?.digest?.slice(0, 16) ?? "",
  );

  const archiveValidated = cli(["validate", archive, "--json"]);
  assert(archiveValidated.payload?.ok === true, "the built archive validates");

  const installed = cli(["install", archive, "--json"]);
  assert(
    installed.payload?.ok === true,
    "CLI install adds the package to the library",
  );

  const v2Source = join(home, "arithmetic-v2-source");
  cpSync(source, v2Source, { recursive: true });
  const v2ManifestPath = join(v2Source, "osmium.json");
  const v2Manifest = JSON.parse(readFileSync(v2ManifestPath, "utf8"));
  v2Manifest.package_version = "0.2.0";
  writeFileSync(v2ManifestPath, `${JSON.stringify(v2Manifest, null, 2)}\n`);
  const v2Archive = join(home, "arithmetic-v2.osmium");
  assert(cli(["build", v2Source, "--output", v2Archive]).payload?.ok === true, "a second explicit version builds");
  assert(cli(["install", v2Archive, "--json"]).payload?.ok === true, "the second version installs without replacing v1");

  const expandedSource = join(options.repo, "examples", "arithmetic-expanded");
  const expandedArchive = join(home, "arithmetic-expanded.osmium");
  assert(cli(["build", expandedSource, "--output", expandedArchive]).payload?.ok === true, "the expanded relation fixture builds");
  assert(cli(["install", expandedArchive, "--json"]).payload?.ok === true, "the expanded relation fixture installs");

  const cardiovascularSource = join(options.repo, "examples", "cardiovascular-atlas-validation");
  const cardiovascularArchive = join(home, "cardiovascular-atlas-validation.osmium");
  const cardiovascularBuild = cli(["build", cardiovascularSource, "--output", cardiovascularArchive]);
  assert(cardiovascularBuild.payload?.ok === true, "the 44-Concept cardiovascular validation Package builds");
  assert(cli(["install", cardiovascularArchive, "--json"]).payload?.ok === true, "the cardiovascular validation Package installs");

  const listed = cli(["packages", "--json"]);
  assert(
    Array.isArray(listed.payload?.data) && listed.payload.data.length === 4,
    "the library lists both arithmetic versions, the expanded fixture, and the cardiovascular Package",
  );

  // 2. Start the application and confirm it renders the installed package.
  const app = new App();
  await app.start();
  assert(
    (await app.text()).includes("ライブラリ"),
    "the application shows the packages panel",
  );
  assert(
    await app.evaluate('document.querySelector(".brand-copy")?.textContent?.trim() === "CosmoOrder"'),
    "the application uses the CosmoOrder product name",
  );
  assert(
    await app.evaluate(
      'document.querySelector(".brand-mark svg path") !== null',
    ),
    "the header keeps the existing crystal mark",
  );
  assert(
    await app.evaluate(
      'document.querySelectorAll(".app-nav svg.lucide").length === 4',
    ),
    "navigation uses Lucide icons",
  );
  assert(
    await app.evaluate(
      '(() => { const widths = [...document.querySelectorAll(".app-nav svg.lucide")].map((icon) => getComputedStyle(icon).getPropertyValue("stroke-width")); return widths.length === 4 && widths.every((width) => Number.parseFloat(width) === 1.9); })()',
    ),
    "navigation icon stroke weights are consistent",
    await app.evaluate(
      '[...document.querySelectorAll(".app-nav svg.lucide")].map((icon) => getComputedStyle(icon).getPropertyValue("stroke-width")).join(", ")',
    ),
  );
  await app.click('.package[data-package-id="org.example/arithmetic"][data-package-version="0.1.0"]', "open explicitly selected version 0.1.0");
  await app.waitForSelector("#lesson-heading", "explicit version 0.1.0 learner view");
  assert((await app.text()).includes("バージョン 0.1.0"), "Library opens version 0.1.0 exactly");
  await app.click(".app-nav button:first-child", "return to Library for explicit version selection");
  await app.click('.package[data-package-id="org.example/arithmetic"][data-package-version="0.2.0"]', "open explicitly selected version 0.2.0");
  await app.waitForSelector("#lesson-heading", "explicit version 0.2.0 learner view");
  assert((await app.text()).includes("バージョン 0.2.0"), "Library opens version 0.2.0 exactly");
  await app.click(".app-nav button:first-child", "return to Library after version selection");

  // Lesson → author-defined Route → Package-local Atlas → prerequisite → Resource → Atlas.
  await app.click('.package[data-package-id="org.example/arithmetic-expanded"][data-package-version="0.1.0"]', "open the expanded Package for the learner navigation experiment");
  await app.waitForSelector("#lesson-heading", "expanded Package Lesson");
  assert((await app.text()).includes("最初の教材を読む"), "the regular Lesson still leads with its first Resource");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click('button[aria-label="学習ルートを見る"]', "zoom from Lesson to Route");
  await app.waitForSelector("#route-heading", "author-defined Route");
  const routeOpenMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await checkPresentation(app, "learning-route");
  await app.click(".route-step:nth-child(3) .route-step-button", "select the Addition learning objective");
  const routeText = await app.text();
  assert(routeText.includes("Add two single-digit numbers"), "Route selects the current objective");
  assert(routeText.includes("Compare two small quantities"), "Route explains the preceding Curriculum objective");
  assert(routeText.includes("Add numbers with a carry"), "Route explains the next Curriculum objective");
  assert(routeText.includes("Counting"), "Route separately exposes the explicit prerequisite Concept");
  assert(routeText.includes("作者が定義した順序") && routeText.includes("明示された前提関係"), "Curriculum and prerequisite semantics have separate labels");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click(".route-atlas-link", "zoom from Route to its local Atlas neighborhood");
  await app.waitForSelector("#atlas-heading", "Package-local Atlas");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "Addition"', "Addition as the Atlas center");
  const atlasOpenMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await checkPresentation(app, "package-atlas");
  const atlasText = await app.text();
  assert(atlasText.includes("Counting") && atlasText.includes("Subtraction") && atlasText.includes("Multiplication"), "Atlas displays bounded incoming and outgoing prerequisite neighbors");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click(".atlas-prerequisite-lane .atlas-neighbor", "select the prerequisite Concept");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "Counting"', "prerequisite Concept becomes the Atlas center");
  const recenterMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  assert((await app.text()).includes("Counting and comparison"), "Atlas shows the selected Concept's associated Resource");
  await app.click('.atlas-detail-grid section[aria-labelledby="atlas-resources-heading"] button.learning-link', "open the associated Resource from Atlas");
  await app.waitForSelector("#reader-heading", "Resource opened from Atlas");
  await app.clickText("目次へ", "return from Resource to the same Atlas context");
  await app.waitForSelector("#atlas-heading", "Atlas restored after Resource");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "Counting"', "Atlas preserves selected Concept after Resource");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.setField('.atlas-search input', "Division");
  await app.waitForSelector(".atlas-search-results button", "backend Concept search results");
  await app.waitFor('() => document.querySelector(".atlas-search-results")?.innerText.includes("Division") === true', "search result title");
  const searchMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await app.click(".atlas-search-results button", "recenter through package-local search");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "Division"', "search result becomes the Atlas center");
  await app.clickText("Lessonへ戻る", "return from Atlas to the ordinary Lesson");
  await app.waitForSelector("#lesson-heading", "Lesson restored after Atlas exploration");
  pass("Lesson → Route → Atlas → prerequisite → Resource → Lesson navigation", `route ${routeOpenMs.toFixed(0)} ms; Atlas ${atlasOpenMs.toFixed(0)} ms; recenter ${recenterMs.toFixed(0)} ms; search ${searchMs.toFixed(0)} ms`);
  await app.click(".app-nav button:first-child", "return to Library after the learner navigation experiment");

  // P1.5: test a realistic, locally scoped 44-Concept medical Package.
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click('[data-package-id="org.example/cardiovascular-atlas-validation"][data-package-version="0.1.0"]', "open the cardiovascular learning Package");
  await app.waitForSelector("#lesson-heading", "44-Concept cardiovascular Lesson");
  const cardiovascularLessonMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  const cardiovascularLesson = await app.evaluate(`(() => ({
    concepts: document.querySelectorAll("article.concept.card").length,
    objectives: document.querySelectorAll(".objective-list > li").length,
    resources: document.querySelectorAll(".curriculum-resources button.learning-link").length,
    assessments: document.querySelectorAll(".objective-list .learning-actions button.learning-link").length,
    curricula: document.querySelectorAll(".curriculum-section").length,
    summary: document.querySelector(".lesson-hero")?.innerText ?? "",
  }))()`);
  assert(cardiovascularLesson.concepts === 10 && cardiovascularLesson.objectives === 10, "Lesson opens one curriculum section and defers the other 34 Concepts/Objectives", `${cardiovascularLesson.concepts} visible Concepts / ${cardiovascularLesson.objectives} visible Objectives`);
  assert(cardiovascularLesson.curricula === 4, "Lesson groups the Package into four authored sections", `${cardiovascularLesson.curricula} sections`);
  assert(cardiovascularLesson.resources === 1, "the shared module Resource is rendered once for the open section", `${cardiovascularLesson.resources} Resource link`);
  assert(cardiovascularLesson.assessments === 1, "only the open section's linked checks are mounted", `${cardiovascularLesson.assessments} Assessment links`);
  assert(await app.evaluate('document.querySelectorAll(".curriculum-section:not([open]) .concept").length === 0'), "collapsed sections do not mount their Concept cards");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click(".concept-prerequisite-link", "open an explicit prerequisite directly from Lesson");
  await app.waitForSelector("#atlas-heading", "direct Lesson-to-Atlas prerequisite view");
  const directPrerequisiteAtlasMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await app.clickText("Lessonへ戻る", "return from direct prerequisite Atlas to Lesson");
  await app.waitForSelector("#lesson-heading", "Lesson after direct prerequisite lookup");
  assert(await app.evaluate('document.querySelector(".lesson-current-route")?.innerText.includes("今") === true && document.querySelector(".lesson-current-route")?.innerText.includes("次") === true'), "Lesson shows compact author-ordered before/current/next context");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click(".curriculum-section:nth-of-type(2) summary", "expand the hemodynamics section");
  await app.waitFor('() => document.querySelectorAll(".curriculum-section:nth-of-type(2) .concept").length > 0', "expanded section topics");
  const sectionExpandMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  assert(await app.evaluate('document.querySelectorAll(".curriculum-section:nth-of-type(2) .curriculum-resources button.learning-link").length === 1'), "shared resource remains deduplicated after expanding its module");
  await app.send("Emulation.setDeviceMetricsOverride", { width: 390, height: 900, deviceScaleFactor: 1, mobile: false });
  assert(await app.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"), "44-Concept Lesson fits a 390px viewport without horizontal overflow");
  await app.send("Emulation.clearDeviceMetricsOverride");

  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click('button[aria-label="学習ルートを見る"]', "open Route for the cardiovascular Package");
  await app.waitForSelector("#route-heading", "the cardiovascular author-defined Route");
  const cardiovascularRouteMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  assert(await app.evaluate('document.querySelectorAll(".route-step").length === 10'), "Route shows the current module's ten ordered objectives", "10 steps in the active Curriculum");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click(".route-atlas-link", "zoom from the cardiovascular Route to the Package-local Atlas");
  await app.waitForSelector("#atlas-heading", "the cardiovascular Package-local Atlas");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "心臓の構造"', "cardiac structure as the Atlas center");
  const cardiovascularAtlasMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  const cardiacHub = await app.evaluate(`(() => ({
    visible: document.querySelectorAll(".atlas-dependent-lane .atlas-neighbor").length,
    overflow: document.querySelector(".atlas-dependent-lane")?.innerText ?? "",
    cap: document.querySelector(".atlas-dependent-lane")?.innerText.includes("他1件は近傍表示の上限外") ?? false,
  }))()`);
  assert(cardiacHub.visible === 6 && cardiacHub.cap, "Atlas bounds the seven outgoing prerequisite neighbors at six and reports one hidden", cardiacHub.overflow);
  await checkPresentation(app, "cardiovascular-44-concept-atlas");
  await app.evaluate('document.querySelector(".atlas-panel .developer-details").open = true');
  const contextBounds = await app.evaluate('document.querySelector(".atlas-panel .developer-details")?.innerText ?? ""');
  assert(contextBounds.includes("3 hops"), "Atlas uses the existing three-hop context query", contextBounds);

  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.setField('.atlas-search input', "心不全という臨床症候群");
  await app.waitForSelector(".atlas-search-results button", "medical Concept search result");
  await app.waitFor('() => document.querySelector(".atlas-search-results")?.innerText.includes("心不全という臨床症候群") === true', "exact syndrome Concept search result");
  const cardiovascularSearchMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click(".atlas-search-results button", "recenter from search to the heart-failure syndrome");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "心不全という臨床症候群"', "heart failure becomes the Atlas center");
  const cardiovascularRecenterMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await app.clickText("Lessonへ戻る", "return from the cardiovascular Atlas to the Lesson");
  await app.waitForSelector("#lesson-heading", "the cardiovascular Lesson after Atlas exploration");
  await app.click(".curriculum-section:nth-of-type(2) summary", "expand hemodynamics for the remediation prototype");
  await app.click('button[aria-label="問題を開く:前負荷の概念に最も近い説明はどれですか？"]', "open the preload assessment");
  await app.click('input[name="response"][value="b"]', "select an incorrect preload response");
  await app.click('button[type="submit"]', "submit the incorrect preload response");
  await app.waitFor('() => document.querySelector(".feedback.incorrect") !== null', "incorrect feedback before remediation");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.clickText("関連教材を読み直して再挑戦", "return to a linked resource after an incorrect response");
  await app.waitForSelector(".pv-prototype", "the allowlisted pressure-volume interaction");
  const remediationResourceMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  await app.clickText("前負荷を増やす", "change the illustrative preload condition");
  await app.waitFor('() => document.querySelector(".pv-observation")?.innerText.includes("右へ移動") === true', "pressure-volume loop observation");
  await app.evaluate("window.__navigationPerfStart = performance.now()");
  await app.click('input[name="pv-answer"][value="preload"]', "answer the interaction's observation check");
  await app.waitFor('() => document.querySelector(".pv-feedback.correct") !== null', "interactive feedback");
  const interactiveFeedbackMs = await app.evaluate("performance.now() - window.__navigationPerfStart");
  assert(await app.evaluate('document.querySelector(".pv-prototype")?.innerText.includes("Packageから任意のスクリプトは実行しません") === true'), "the interactive prototype states its inert-package and no-history boundary");
  await app.clickText("もう一度", "reset the interactive prototype for retry");
  await app.click(".reader-nav button:nth-child(2)", "return from Resource review to the unanswered assessment");
  await app.waitForSelector("#assessment-heading", "assessment restored for a fresh retry");
  await app.click('input[name="response"][value="a"]', "choose the corrected preload response");
  await app.click('button[type="submit"]', "submit the corrected response");
  await app.waitFor('() => document.querySelector(".feedback.correct") !== null', "correct retry feedback");
  await app.click('button[aria-label="履歴"]', "inspect events after remediation retry");
  await app.waitForSelector("#history-heading", "remediation attempt history");
  await app.waitFor('() => document.querySelectorAll(".history-list > li").length === 2', "both incorrect and corrected answers are preserved");
  await app.clickText("目次へ", "return to Lesson after confirming both attempt events");
  await app.waitForSelector("#lesson-heading", "Lesson restored after remediation");
  await app.click(".curriculum-section:nth-of-type(3) summary", "expand the heart-failure section for the relation prototype");
  await app.waitForSelector(".relation-prototype", "the explicitly non-prerequisite heart failure–RAAS relation prototype");
  await app.clickText("RAASを見る", "open the related RAAS topic");
  await app.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "レニン・アンジオテンシン・アルドステロン系（RAAS）"', "relation prototype opens RAAS without changing prerequisite data");
  await app.clickText("Lessonへ戻る", "return from related-topic prototype");
  await app.waitForSelector("#lesson-heading", "Lesson after relation prototype");
  pass("P2 Lesson navigation and prototypes", `Lesson ${cardiovascularLessonMs.toFixed(0)} ms; expand ${sectionExpandMs.toFixed(0)} ms; direct Atlas ${directPrerequisiteAtlasMs.toFixed(0)} ms; Route ${cardiovascularRouteMs.toFixed(0)} ms; Atlas ${cardiovascularAtlasMs.toFixed(0)} ms; search ${cardiovascularSearchMs.toFixed(0)} ms; recenter ${cardiovascularRecenterMs.toFixed(0)} ms; remediation Resource ${remediationResourceMs.toFixed(0)} ms; interaction feedback ${interactiveFeedbackMs.toFixed(0)} ms`);
  await app.click(".app-nav button:first-child", "return to Library after cardiovascular Package validation");

  // One-Concept package: Atlas presents a calm empty state rather than fabricating edges.
  await app.click('.package[data-package-id="org.example/arithmetic"][data-package-version="0.1.0"]', "open the sparse package");
  await app.waitForSelector("#lesson-heading", "sparse package Lesson");
  await app.click('button[aria-label="学習ルートを見る"]', "open Route for the sparse package");
  await app.waitForSelector("#route-heading", "sparse Route");
  await app.click(".route-atlas-link", "open Atlas with no prerequisite relations");
  await app.waitForSelector("#atlas-heading", "sparse Package Atlas");
  await app.waitFor('() => document.querySelector(".atlas-neighborhood .empty") !== null', "sparse Atlas empty state");
  assert((await app.text()).includes("前提関係はまだ定義されていません"), "sparse Package does not fabricate prerequisite relations");
  await app.clickText("Lessonへ戻る", "return from sparse Atlas to Lesson");
  await app.waitForSelector("#lesson-heading", "sparse Lesson restored");
  pass("one-Concept/no-relation Package has a useful Atlas empty state");
  await app.click(".app-nav button:first-child", "return to Library before authoring regression coverage");

  // Source picker → real Tauri command → Core diagnostics → authoring view.
  const authoringFixture = (name) => join(
    options.repo,
    "apps",
    "desktop",
    "src-tauri",
    "tests",
    "fixtures",
    name,
  );
  await app.evaluate(`window.osmiumE2eSourceDirectory = ${JSON.stringify(authoringFixture("authoring-valid"))}`);
  await app.click('button[aria-label="作成者レビュー"]', "open authoring review");
  await app.clickText("新しい教材を作成", "start a new source package");
  await captureScreenshot(app, "new-package-form");
  assert(
    await app.evaluate(`(() => {
      const options = [...(document.querySelector('.authoring-form select')?.options ?? [])].map((option) => option.value);
      return ['ja', 'en', 'zh', 'es', 'fr', 'de', 'ko', 'pt', 'hi', 'ar'].every((code) => options.includes(code));
    })()`),
    "new package language uses a common-language selector",
  );
  assert(
    await app.evaluate('[...document.querySelectorAll(".authoring-form button")].some((button) => button.textContent.trim() === "変更")'),
    "new package destination has a friendly change action",
  );
  const ordinaryFormText = await app.evaluate('document.querySelector(".authoring-form")?.innerText ?? ""');
  assert(!/Package ID|namespace|reverse.domain|BCP.?47|[A-Z]:\\\\/i.test(ordinaryFormText), "ordinary users are not shown package internals or raw paths");
  const guiSource = join(home, "gui-authoring-source");
  await app.evaluate(`window.osmiumE2eNewSourceDirectory = ${JSON.stringify(guiSource)}`);
  assert(
    (await app.evaluate('window.osmiumE2eNewSourceDirectory')) === guiSource,
    "isolated E2E source destination is injected before create",
    guiSource,
  );
  await app.evaluate(`(() => {
    const fields = [...document.querySelectorAll('.authoring-form input')];
    const title = fields.find((field) => field.type !== 'text' || field.autocomplete !== 'off') ?? fields[0];
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(title, 'GUI作成教材');
    title.dispatchEvent(new Event('input', { bubbles: true }));
    const language = document.querySelector('.authoring-form select');
    language.value = 'ja';
    language.dispatchEvent(new Event('change', { bubbles: true }));
  })()`);
  await app.clickText("作成", "create the GUI source package");
  await app.waitFor('() => document.querySelector(".authoring-form textarea") !== null', "new package editor");
  await app.evaluate('document.querySelector(".source-review-summary .authoring-internals")?.setAttribute("open", "")');
  const generatedId = await app.evaluate('document.querySelector(".source-review-summary .authoring-internals dd code")?.textContent ?? ""');
  assert(/^org\.osmium\.generated\/[a-z][a-z0-9-]*-[0-9a-f]{32}$/.test(generatedId), "package ID is generated with a collision-resistant suffix and visible in Advanced", generatedId);
  const advancedPath = await app.evaluate('document.querySelector(".source-review-summary .authoring-internals dd:nth-of-type(3) code")?.textContent ?? ""');
  assert(advancedPath.includes(guiSource), "absolute source path is available in Advanced", advancedPath);
  await captureScreenshot(app, "authoring-editor");
  assert((await app.text()).includes("未チェック"), "new package starts in the Not checked state");
  await app.clickText("教材をチェック", "validate the untouched GUI scaffold");
  await app.waitFor('() => document.querySelector(".source-review-summary")?.innerText.includes("注意点あり") === true', "initial Core validation");
  assert((await app.text()).includes("注意点あり"), "warning-only validation is shown as Warnings");
  await app.setField('.authoring-form > label input', 'GUI作成教材（編集済み）');
  await app.setField('[aria-labelledby="concepts-heading"] .authoring-entity input', 'GUIの基礎');
  await app.setField('[aria-labelledby="objectives-heading"] .authoring-entity input', '基本の操作ができる');
  await app.setField('[aria-labelledby="resources-heading"] .authoring-entity input', 'Markdown resource title');
  await app.setField('[aria-labelledby="resources-heading"] .authoring-entity textarea', '# GUI lesson\\n\\nCreated and saved in the desktop editor.');

  // Build a second concept/objective/resource and make the relations visible
  // in the authored package; the UI only edits semantic names, while Core
  // allocates stable IDs and validates the resulting graph.
  await app.clickText("テーマを追加", "add a second topic");
  await app.setField('[aria-labelledby="concepts-heading"] .authoring-entity:last-of-type input', 'GUIの応用');
  await app.evaluate(`(() => { const rows = document.querySelectorAll('[aria-labelledby="concepts-heading"] .authoring-entity'); rows[1]?.querySelector('input[type=checkbox]')?.click(); })()`);
  await app.clickText("学習目標を追加", "add a second learning goal");
  await app.setField('[aria-labelledby="objectives-heading"] .authoring-entity:last-of-type input', '応用操作ができる');
  await app.setField('[aria-labelledby="objectives-heading"] .authoring-entity:last-of-type select', 'new:unused');
  await app.evaluate(`(() => { const select = document.querySelector('[aria-labelledby="objectives-heading"] .authoring-entity:last-of-type select'); const option = [...(select?.options ?? [])].find((item) => item.textContent === 'GUIの応用'); if (select && option) { select.value = option.value; select.dispatchEvent(new Event('change', { bubbles: true })); } })()`);
  await app.clickText("教材コンテンツを追加", "add a second learning resource");
  await app.setField('[aria-labelledby="resources-heading"] .authoring-entity:last-of-type input', '応用の教材');
  await app.setField('[aria-labelledby="resources-heading"] .authoring-entity:last-of-type textarea', '# 応用\\n\\n二つ目の教材コンテンツです。');
  await app.evaluate(`(() => { const rows = document.querySelectorAll('[aria-labelledby="resources-heading"] .authoring-entity'); rows[1]?.querySelectorAll('input[type=checkbox]')[1]?.click(); })()`);

  await app.evaluate(`(() => { const curricula = document.querySelectorAll('[aria-labelledby="curricula-heading"] .authoring-entity'); curricula[0]?.querySelectorAll('fieldset input[type=checkbox]')[1]?.click(); })()`);
  await app.evaluate(`(() => { const rows = document.querySelectorAll('[aria-labelledby="curricula-heading"] .authoring-entity .curriculum-order li'); rows[1]?.querySelector('button[aria-label="上へ"]')?.click(); })()`);
  await app.evaluate(`(() => { document.querySelector('[aria-labelledby="curricula-heading"] .authoring-entity .curriculum-order li:first-child button[aria-label="下へ"]')?.click(); })()`);

  await app.clickText("選択問題", "add a single-select question");
  await app.setField('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type textarea', '1 + 1 はいくつですか？');
  await app.setField('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type .assessment-option-edit input', '2');
  await app.setField('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type .assessment-option-edit:last-of-type input', '3');
  await app.setField('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type > label:last-of-type textarea', '1に1を足すと2です。');
  await app.evaluate(`(() => { document.querySelectorAll('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type fieldset:last-of-type input[type=checkbox]')[1]?.click(); })()`);
  await app.clickText("○×問題", "add a boolean question");
  await app.setField('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type textarea', '1 + 1 は 2 です。');
  await app.setField('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type > label:last-of-type textarea', 'その通りです。');
  await app.evaluate(`(() => { document.querySelectorAll('[aria-labelledby="assessments-heading"] .authoring-entity:last-of-type fieldset:last-of-type input[type=checkbox]')[1]?.click(); })()`);
  const editedState = await app.evaluate(`(() => ({
    text: document.querySelector(".source-review-summary")?.innerText ?? "",
    buttons: [...document.querySelectorAll(".source-review-summary button")].map((button) => ({ label: button.innerText.trim(), disabled: button.disabled })),
  }))()`);
  assert(editedState.text.includes("変更後に未チェック"), "editing after validation invalidates the previous result");
  assert(editedState.buttons.some((button) => button.label === "保存" && !button.disabled), "Save becomes available for dirty edits");
  assert(editedState.buttons.some((button) => button.label === "教材をチェック" && button.disabled), "Check stays disabled until dirty edits are saved");
  assert(editedState.buttons.some((button) => button.label.includes("ライブラリに追加") && button.disabled), "Install is blocked while edits are dirty");
  await app.clickText("保存", "save the GUI lesson");
  await app.waitFor('() => { const section = document.querySelector(".source-review-summary"); return section !== null && section.innerText.includes("未チェック") && !section.innerText.includes("保存中…") && [...section.querySelectorAll("button")].some((button) => button.textContent.trim() === "保存" && button.disabled); }', "saved edits require fresh validation");
  await app.clickText("教材をチェック", "validate the GUI source");
  await app.waitFor('() => document.querySelector(".source-review-summary")?.innerText.includes("注意点あり") === true', "fresh Core validation completes");
  assert(await app.evaluate('document.querySelector(".source-review-summary")?.dataset.valid === "true" && document.querySelector(".source-review-summary")?.innerText.includes("未保存の変更") === false'), "GUI-created package validates with Core");
  const installReadiness = await app.evaluate(`(() => { const button = [...document.querySelectorAll("button")].find((item) => item.textContent.trim() === "ライブラリに追加して学ぶ"); return { state: document.querySelector(".source-review-summary")?.innerText ?? "", valid: document.querySelector(".source-review-summary")?.dataset.valid ?? "", disabled: button?.disabled ?? null, diagnostics: document.querySelector(".authoring-review")?.innerText ?? "" }; })()`);
  assert(installReadiness.disabled === false, "fresh Core validation enables install");
  assert(!installReadiness.diagnostics.includes("OSM_LINT_OBJECTIVE_NOT_IN_CURRICULUM") && !installReadiness.diagnostics.includes("OSM_LINT_OBJECTIVE_NO_RESOURCE"), "objective, resource, and curriculum relations validate");
  await app.waitFor('() => [...document.querySelectorAll("button")].some((button) => button.textContent.trim() === "ライブラリに追加して学ぶ" && !button.disabled)', "install action is enabled after validation");
  const exportedFile = join(home, "gui-created.osmium");
  await app.evaluate(`window.osmiumE2eExportDestination = ${JSON.stringify(exportedFile)}`);
  await app.clickText("教材ファイルを書き出す", "export the GUI package as .osmium");
  await app.waitFor(`() => document.querySelector(".notice")?.innerText.includes(${JSON.stringify(exportedFile)}) === true`, "exported distribution status");
  assert(existsSync(exportedFile), ".osmium export writes the Core-built artifact");
  await app.clickText("ライブラリに追加して学ぶ", "install and preview the GUI package");
  await app.waitFor('() => document.querySelector(".curriculum") !== null', "installed package learner view");
  await captureScreenshot(app, "learner-preview");
  assert((await app.text()).includes("GUI作成教材（編集済み）"), "GUI-installed package metadata opens from learner runtime");
  assert((await app.text()).includes("GUIの基礎"), "GUI-edited concept title appears in learner view");
  await app.clickText("Markdown resource title", "open the installed GUI lesson resource");
  await app.waitFor('() => document.querySelector(".reader") !== null', "installed Markdown resource reader");
  assert((await app.text()).includes("Created and saved in the desktop editor."), "learner reader shows the installed resource content");
  await app.clickText("目次へ", "return to the authored curriculum");
  await app.waitForSelector("#lesson-heading", "authored curriculum after reading");
  assert((await app.text()).includes("応用の教材"), "multiple resources and their objective relation reach the learner view");
  await app.click('button[aria-label="教材を開く:応用の教材"]', "open the second authored resource");
  await app.waitForSelector("#reader-heading", "second authored resource reader");
  assert((await app.text()).includes("二つ目の教材コンテンツです。"), "second resource body is installed and readable");
  await app.clickText("目次へ", "return to the authored curriculum after the second resource");
  await app.waitForSelector("#lesson-heading", "authored curriculum before answering");
  await app.click('button[aria-label="問題を開く:1 + 1 はいくつですか？"]', "open the authored single-select assessment");
  await app.waitForSelector("#assessment-heading", "authored single-select question");
  assert((await app.text()).includes("1 + 1 はいくつですか？"), "the GUI-authored single-select stimulus reaches the learner");
  await app.click(".option:first-of-type .option-text", "select the authored correct option");
  await app.click('button[type="submit"]', "grade the authored multiple-choice answer");
  await app.waitFor('() => document.body.innerText.includes("1に1を足すと2です。")', "authored answer feedback");
  await app.clickText("次の問題", "continue to the authored boolean assessment");
  await app.waitFor('() => document.body.innerText.includes("1 + 1 は 2 です。")', "authored boolean question");
  await app.click('.option:has(input[value="true"])', "select authored boolean answer");
  await app.click('button[type="submit"]', "grade the authored boolean answer");
  await app.waitFor('() => document.body.innerText.includes("その通りです。")', "authored boolean feedback");
  pass("GUI-authored assessments evaluate through the Learner/Core flow");
  await app.click('button[aria-label="進捗"]', "open progress for the authored objectives");
  await app.waitForSelector("#progress-heading", "authored objective progress");
  assert((await app.text()).includes("応用操作ができる"), "authored objective relation appears in progress");
  await app.click('button[aria-label="履歴"]', "open history for authored answers");
  await app.waitForSelector("#history-heading", "authored learning history");
  await app.waitFor('() => document.querySelectorAll(".history-list > li").length === 2', "two authored assessment events");
  pass("the authored package records both attempts in History");
  await app.click(".app-nav button:first-child", "return to library after GUI preview");
  await app.evaluate(`window.osmiumE2eDistributionPath = ${JSON.stringify(exportedFile)}`);
  await app.clickText("教材を追加", "open the Library add menu");
  await app.clickText("CosmoOrder教材ファイルを追加", "import an exported .osmium package");
  await app.waitFor('() => document.querySelector(".curriculum") !== null', "imported distribution opens in learner view");
  pass("Desktop imports the exported .osmium through Runtime.install");
  if (options.authoringOnly) {
    await app.stop();
    pass("normal desktop build completed create, edit, save, validate, install and learner preview");
    console.log(`\n${steps.length} checks passed.`);
    return;
  }
  await app.click('button[aria-label="作成者レビュー"]', "return to authoring for source fixture reviews");

  await app.clickText("編集用フォルダーを開く", "select a valid source package");
  await app.waitFor(
    '() => document.querySelector(".source-review-summary")?.dataset.valid === "true"',
    "valid source package review",
  );
  assert(
    await app.evaluate('document.querySelector(".authoring-review") !== null && document.querySelector(".authoring-review [data-severity=error]") === null'),
    "valid source reaches AuthoringReview without errors",
  );
  await app.evaluate('document.querySelector(".source-review-summary .authoring-internals")?.setAttribute("open", "")');
  assert((await app.text()).includes("org.example/authoring-review"), "Advanced authoring details display the source package ID");
  await app.evaluate('document.querySelector(".source-review-summary .authoring-internals")?.removeAttribute("open")');
  await app.evaluate(`window.osmiumE2eSourceDirectory = ${JSON.stringify(authoringFixture("authoring-broken"))}`);
  await app.clickText("編集用フォルダーを開く", "select a broken source package");
  await app.waitFor(
    '() => Array.from(document.querySelectorAll(".authoring-review li")).some((item) => item.innerText.includes("OSM_REFERENCE"))',
    "broken source diagnostic projection",
  );
  const authoringDiagnostic = await app.evaluate('Array.from(document.querySelectorAll(".authoring-review li")).find((item) => item.innerText.includes("OSM_REFERENCE"))?.innerText ?? ""');
  assert(
    authoringDiagnostic.includes("entities/objectives.json") && authoringDiagnostic.includes("/0/concept"),
    "AuthoringReview preserves Core diagnostic file and path",
    authoringDiagnostic,
  );
  assert(!(await app.text()).includes("ライブラリに追加して学ぶ"), "invalid source has no learner install action");
  await app.click(".app-nav button:first-child", "return to learner library");
  assert(
    await app.evaluate('document.querySelector(".authoring") === null'),
    "authoring review is not rendered in the learner library",
  );

  await app.waitFor(
    '() => document.querySelector(".package:not(:disabled)") !== null',
    "the installed package in the list",
  );
  pass("the installed package is listed in the UI", PACKAGE_ID);
  await app.click('button[aria-label="English"]', "switch UI language to English");
  const englishLibrary = await app.text();
  assert(englishLibrary.includes("Library"), "English UI strings are active");
  assert(
    englishLibrary.includes("足し算の基礎"),
    "package content language stays Japanese when the UI is English",
  );
  await app.click('button[aria-label="日本語"]', "switch UI language back to Japanese");
  assert(
    await app.evaluate('(() => { const text = document.querySelector(".package-counts")?.innerText ?? ""; return ["テーマ", "学習目標", "教材", "問題"].every((label) => text.includes(label)); })()'),
    "the library card shows all four package counts",
  );

  assert(
    !(await app.text()).includes(PACKAGE_ID),
    "library metadata is collapsed",
  );
  await app.click('.library-card:has([data-package-id="org.example/arithmetic"]) summary', "library details");
  assert(
    (await app.text()).includes(PACKAGE_ID),
    "library details expose package metadata",
  );
  await app.click('.library-card:has([data-package-id="org.example/arithmetic"]) summary', "close library details");
  await checkPresentation(app, "library");

  // 3. Open the package, then the curriculum and concept.
  await app.click('.package[data-package-id="org.example/arithmetic"][data-package-version="0.1.0"]', "the installed package");
  await app.waitForSelector("#lesson-heading", "the lesson panel");
  const lessonText = await app.text();
  assert(
    lessonText.includes("足し算"),
    "the curriculum / concept view renders",
    "concept 足し算",
  );
  assert(
    lessonText.includes(GOLDEN_OBJECTIVE),
    "the objective is listed",
    GOLDEN_OBJECTIVE,
  );
  assert(
    ["テーマ", "学習目標", "このセクションの教材", "問題 · 確かめる"].every((label) => lessonText.includes(label)),
    "the lesson shows the authored package hierarchy",
  );

  assert(
    await app.evaluate(
      'document.querySelector(".app-nav [aria-current=page]").getAttribute("aria-label") === "目次を表示"',
    ),
    "shell identifies current course",
  );
  await checkPresentation(app, "curriculum");

  // 4. Read the Markdown resource.
  await app.click('button[aria-label^="教材を開く"]', "the lesson resource");
  await app.waitForSelector("#reader-heading", "the reader panel");
  await app.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(GOLDEN_MARKER)})`,
    "the Markdown body",
  );
  const readerText = await app.text();
  assert(
    readerText.includes(GOLDEN_MARKER),
    "the Markdown lesson body is readable",
    GOLDEN_MARKER,
  );
  assert(readerText.includes("足し算"), "the lesson heading is rendered");

  assert(
    await app.evaluate('document.querySelectorAll(".reader-nav").length === 2'),
    "reader has navigation before and after content",
  );
  await checkPresentation(app, "reader");

  // 5. Answer the assessment. The reader and the outline are separate panels,
  //    so the outline has to be shown again before an item can be opened.
  await app.clickText("目次へ", "back to the outline");
  await app.waitForSelector("#lesson-heading", "the lesson panel again");
  await app.click('button[aria-label^="問題を開く"]', "the assessment");
  await app.waitForSelector("#assessment-heading", "the assessment panel");
  await app.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(GOLDEN_STIMULUS)})`,
    "the stimulus",
  );
  pass("the assessment stimulus renders", GOLDEN_STIMULUS);

  assert(
    await app.evaluate(
      'document.querySelector("button[type=submit]").disabled',
    ),
    "grading requires a selection",
  );
  await app.click(
    '.option:has(input[value="b"]) .option-text',
    "the correct option card text",
  );
  assert(
    await app.evaluate(
      'document.querySelector("input[value=b]").checked && document.querySelector(".option.selected") !== null',
    ),
    "clicking an option card selects its radio",
  );
  await checkPresentation(app, "assessment");
  await app.click('button[type="submit"]', "the grade button");
  await app.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(GOLDEN_FEEDBACK)})`,
    "the graded feedback",
  );
  const graded = await app.text();
  assert(
    graded.includes("正解"),
    "deterministic grading reports the answer as correct",
  );
  assert(
    !graded.includes("org.osmium.exact.v1"),
    "evaluation metadata is collapsed",
  );
  await app.click(".assessment summary", "assessment details");
  assert(
    (await app.text()).includes("org.osmium.exact.v1"),
    "evaluation engine is available in details",
  );
  await app.click(".assessment summary", "close assessment details");
  await checkPresentation(app, "feedback");
  assert(
    graded.includes(GOLDEN_FEEDBACK),
    "the feedback is rendered from the package",
    GOLDEN_FEEDBACK,
  );

  await app.clickText("次の問題", "next problem after feedback");
  await app.waitFor(
    '() => document.body.innerText.includes("2 + 1")',
    "boolean question",
  );
  assert(
    await app.evaluate(
      'document.querySelectorAll("input[type=radio]:checked").length === 0 && document.querySelector("button[type=submit]").disabled',
    ),
    "next problem clears the previous selection",
  );
  await app.click('.option:has(input[value="true"])', "boolean answer card");
  assert(
    await app.evaluate('document.querySelector("input[value=true]").checked'),
    "boolean shares the selection card interaction",
  );
  // Keyboard arrows move within the native radio group, and Space selects it.
  await app.evaluate('document.querySelector("input[value=true]").focus()');
  await app.send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key: "ArrowRight",
    code: "ArrowRight",
    windowsVirtualKeyCode: 39,
  });
  await app.send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key: "ArrowRight",
    code: "ArrowRight",
    windowsVirtualKeyCode: 39,
  });
  assert(
    await app.evaluate('document.querySelector("input[value=false]").checked'),
    "keyboard can change the radio selection",
  );
  await app.click('button[type="submit"]', "grade boolean answer");
  await app.waitFor(
    '() => document.querySelector(".feedback.incorrect") !== null',
    "boolean feedback",
  );
  assert(
    (await app.text()).includes("不正解"),
    "incorrect grading shows its result",
  );
  await checkPresentation(app, "incorrect-feedback");
  pass("boolean grading reports the incorrect selection");

  // 6. Progress and history reflect the saved event. Both panels are reachable
  //    from the graded item as well as from the outline.
  await app.click('button[aria-label="進捗"]', "the progress button");
  await app.waitForSelector("#progress-heading", "the progress panel");
  const progressText = await app.text();
  assert(
    progressText.includes(GOLDEN_OBJECTIVE),
    "progress lists the measured objective",
    GOLDEN_OBJECTIVE,
  );
  assert(
    progressText.includes("50%"),
    "progress counts correct attempts",
    "50%",
  );

  assert(
    !progressText.includes("イベントから再構築"),
    "maintenance is collapsed",
  );
  await checkPresentation(app, "progress");
  await app.click('button[aria-label="履歴"]', "the history button");
  await app.waitForSelector("#history-heading", "the history panel");
  await app.waitFor(
    '() => document.querySelectorAll(".history-list > li").length === 2',
    "the learning event",
  );
  pass("the learning event is recorded in the history");

  await app.click(".app-nav button:first-child", "return to Library to remove only version 0.2.0");
  await app.evaluate("window.osmiumE2eConfirmRemoval = true");
  await app.click('li.library-card:has([data-package-version="0.2.0"]) button.package-remove', "remove only the selected package version");
  await app.waitFor('() => document.querySelector(\'li.library-card:has([data-package-id="org.example/arithmetic"][data-package-version="0.2.0"])\') === null && document.querySelector(\'li.library-card:has([data-package-id="org.example/arithmetic"][data-package-version="0.1.0"])\') !== null', "version 0.2.0 removed while 0.1.0 remains");
  pass("Desktop uninstall removes only the selected version");

  assert(
    !(await app.text()).includes("addition.01"),
    "history emphasizes the question rather than its ID",
  );
  await checkPresentation(app, "history");

  // 7. Offline proof: only local assets and the IPC channel were requested.
  const external = app.requests.filter(
    (url) =>
      !url.startsWith("http://tauri.localhost") &&
      !url.startsWith("ipc:") &&
      !url.startsWith("http://ipc.localhost") &&
      !url.startsWith("data:") &&
      !url.startsWith("blob:"),
  );
  assert(
    external.length === 0,
    "the application performs no network request",
    `${app.requests.length} requests, all local`,
  );
  assert(
    app.consoleErrors.length === 0,
    "the webview logged no error",
    app.consoleErrors.join(" | "),
  );

  // 8. Fully stop, then restart and confirm the state survived.
  await app.stop();
  pass("the application shut down completely");

  const afterShutdown = cli(["history", PACKAGE_ID, "--json"]);
  assert(
    Array.isArray(afterShutdown.payload?.data) &&
      afterShutdown.payload.data.length === 2,
    "the event log holds two attempts after shutdown",
  );

  const restarted = new App();
  await restarted.start();
  await restarted.waitFor(
    '() => document.querySelector(".package:not(:disabled)") !== null',
    "the installed package after restart",
  );
  pass("the installed package is still listed after restart", PACKAGE_ID);

  await restarted.click('.package[data-package-id="org.example/arithmetic"][data-package-version="0.1.0"]', "the installed package v0.1.0 after restart");
  await restarted.waitForSelector(
    "#lesson-heading",
    "the lesson panel after restart",
  );
  const restartedLesson = await restarted.text();
  assert(
    restartedLesson.includes("2 回の回答"),
    "progress is restored on the lesson view",
    "2 answers recorded",
  );

  await restarted.click(
    'button[aria-label="進捗"]',
    "the progress button after restart",
  );
  await restarted.waitForSelector(
    "#progress-heading",
    "the progress panel after restart",
  );
  const restartedProgress = await restarted.text();
  assert(
    restartedProgress.includes(GOLDEN_OBJECTIVE),
    "the objective survives the restart",
  );
  assert(
    restartedProgress.includes("50%"),
    "the projected accuracy survives the restart",
    "50%",
  );

  await restarted.clickText("目次へ", "back to the outline after restart");
  await restarted.waitForSelector(
    "#lesson-heading",
    "the outline after restart",
  );
  await restarted.click(
    'button[aria-label="履歴"]',
    "the history button after restart",
  );
  await restarted.waitForSelector(
    "#history-heading",
    "the history panel after restart",
  );
  await restarted.waitFor(
    '() => document.querySelectorAll(".history-list > li").length === 2',
    "the learning event after restart",
  );
  pass("the learning event survives the restart");

  await restarted.click(".app-nav button:first-child", "return to Library to reopen the GUI-authored package");
  await restarted.waitFor(
    `() => document.querySelector('[data-package-id=${JSON.stringify(generatedId)}][data-package-version="0.1.0"]') !== null`,
    "the GUI-authored version after restart",
  );
  await restarted.click(`[data-package-id=${JSON.stringify(generatedId)}][data-package-version="0.1.0"]`, "open the GUI-authored package after restart");
  await restarted.waitForSelector("#lesson-heading", "GUI-authored package learner view after restart");
  await restarted.click('button[aria-label="進捗"]', "open GUI-authored progress after restart");
  await restarted.waitForSelector("#progress-heading", "GUI-authored progress after restart");
  assert((await restarted.text()).includes("応用操作ができる"), "GUI-authored progress survives restart");
  await restarted.click('button[aria-label="履歴"]', "open GUI-authored history after restart");
  await restarted.waitForSelector("#history-heading", "GUI-authored history after restart");
  await restarted.waitFor('() => document.querySelectorAll(".history-list > li").length === 2', "GUI-authored events survive restart");
  pass("GUI-authored assessment events and progress survive restart");

  await restarted.stop();
  pass("the application shut down again");

  // 9. Render all four domain pressure-test packages in the real WebView.
  const domains = ["medicine", "mathematics", "language", "programming"];
  for (const domain of domains) {
    const sourcePath = join(options.repo, "examples", `${domain}-pressure-test`);
    const outputPath = join(home, `${domain}-pressure-test.osmium`);
    const domainBuild = cli(["build", sourcePath, "--output", outputPath]);
    assert(domainBuild.payload?.ok === true, `${domain}: package builds`);
    const domainInstall = cli(["install", outputPath, "--json"]);
    assert(domainInstall.payload?.ok === true, `${domain}: package installs`);
  }

  const rendererApp = new App();
  await rendererApp.start();
  async function openFixture(title) {
    await rendererApp.clickText("ライブラリ", "open package library");
    await rendererApp.waitFor(
      `() => [...document.querySelectorAll(".library-card h2")].some((heading) => heading.textContent.trim() === ${JSON.stringify(title)})`,
      `${title} appears in the library`,
    );
    await rendererApp.waitFor(
      `() => [...document.querySelectorAll(".library-card")].some((card) => card.querySelector("h2")?.textContent.trim() === ${JSON.stringify(title)} && card.querySelector("button.package:not(:disabled)") !== null)`,
      `${title} is ready to open`,
    );
    const opened = await rendererApp.evaluate(`(() => {
      const card = [...document.querySelectorAll(".library-card")].find(
        (candidate) => candidate.querySelector("h2")?.textContent.trim() === ${JSON.stringify(title)},
      );
      const button = card?.querySelector("button.package");
      button?.click();
      return button !== undefined;
    })()`);
    assert(opened, `open ${title}: package is listed`);
    await rendererApp.waitForSelector("#lesson-heading", `${title} lesson`);
  }

  const medicineTitle = "成人の脱水・体液状態評価の基礎";
  await openFixture(medicineTitle);
  const medicineOverview = await rendererApp.text();
  for (const japanese of [
    "体液の基礎から所見の統合へ",
    "体液区分と水の移動",
    "水分・電解質の喪失と体液量の変化",
    "身体所見と経時変化の読み取り",
    "複数の情報から体液状態を考える",
    "細胞内液・間質液・血漿の位置関係を説明し",
    "体液状態について妥当な推論と未確定事項",
  ]) {
    assert(medicineOverview.includes(japanese), `medicine overview shows Japanese metadata: ${japanese}`);
  }
  for (const legacy of [
    "Fluid balance",
    "Vital signs",
    "Explain intake and output",
    "Interpret a vital-sign trend in context",
    "Original fictional pressure-test text",
    "架空ケースで、最初に行う推論として最も適切なのはどれですか？",
  ]) {
    assert(!medicineOverview.includes(legacy), `medicine overview omits legacy text: ${legacy}`);
  }

  await rendererApp.clickText("体液はどこに分布するか", "open the body-compartment resource");
  await rendererApp.waitForSelector(".markdown table", "medicine compartment table");
  assert(
    await rendererApp.evaluate('document.querySelector(".table-scroll") !== null'),
    "medicine table is in a scrollable wrapper",
  );
  const compartmentText = await rendererApp.text();
  assert(compartmentText.includes("細胞内液"), "resource body is Japanese learning content");
  assert(!compartmentText.includes("Original fictional pressure-test text"), "resource omits pressure-test attribution");
  await rendererApp.click(".resource-references summary", "expand resource references");
  const publicSources = await rendererApp.evaluate(`(() => ({
    text: document.querySelector(".resource-references")?.innerText ?? "",
    links: document.querySelectorAll(".resource-references a").length,
    visible: [...(document.querySelectorAll(".resource-references li") ?? [])].map(item => item.dataset.sourceVisibility),
  }))()`);
  assert(publicSources.text.includes("体液区分（OpenStax『解剖生理学』第26.1節）"), "public source title appears from source_ids");
  assert(publicSources.text.includes("https://openstax.org/books/anatomy-and-physiology-2e/pages/26-1-body-fluids-and-fluid-compartments"), "public source locator is shown as text");
  assert(publicSources.links === 0, "source locator is not a Package-controlled navigation link");
  assert(publicSources.visible.includes("public"), "public visibility reaches the Resource view");
  await rendererApp.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await rendererApp.evaluate('document.querySelector(".display-settings").open = true');
  await rendererApp.clickText("200%", "enlarge medicine table view");
  assert(
    await rendererApp.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"),
    "medicine table stays within a 390px viewport at 200% text size",
  );
  await rendererApp.send("Emulation.clearDeviceMetricsOverride");
  await rendererApp.clickText("目次へ", "return to medicine outline");

  await rendererApp.clickText("症例の情報を統合する", "open the case-integration resource");
  await rendererApp.click(".resource-references summary", "expand integration references");
  const sourceVisibility = await rendererApp.evaluate(`(() => ({
    text: document.querySelector(".resource-references")?.innerText ?? "",
    items: [...document.querySelectorAll(".resource-references li")].map(item => ({
      text: item.innerText,
      visibility: item.dataset.sourceVisibility,
      hasLink: item.querySelector("a") !== null,
    })),
  }))()`);
  const niceSource = sourceVisibility.items.find(item => item.visibility === "attribution_only");
  assert(niceSource !== undefined, "attribution-only source is present for its Resource");
  assert(niceSource.text.includes("臨床指針CG174"), "attribution citation is shown");
  assert(!niceSource.text.includes("https://www.nice.org.uk/guidance/cg174"), "attribution locator is hidden");
  assert(
    await rendererApp.evaluate('!document.documentElement.outerHTML.includes("https://www.nice.org.uk/guidance/cg174")'),
    "attribution locator is absent from the DOM",
  );
  assert(!niceSource.hasLink, "attribution source has no link");
  assert(!sourceVisibility.text.includes("執筆者向け非公開メモ"), "private source title is absent");
  assert(!sourceVisibility.text.includes("制作記録。配布・学習者表示の対象外。"), "private source citation is absent");
  assert(!sourceVisibility.text.includes("../../docs/MEDICINE_PACKAGE_REVIEW.md"), "private source locator is absent from visible references");
  assert(
    await rendererApp.evaluate('!document.documentElement.outerHTML.includes("../../docs/MEDICINE_PACKAGE_REVIEW.md")'),
    "private source locator is absent from the DOM",
  );
  await rendererApp.clickText("目次へ", "return to medicine curriculum");

  const medicineAssessmentText = "成人が数日間の水様便と摂取低下";
  await rendererApp.clickTextStartingWith(medicineAssessmentText, "open the medicine assessment");
  await rendererApp.waitForSelector("#assessment-heading", "medicine assessment");
  const assessmentText = await rendererApp.text();
  assert(assessmentText.includes("血清ナトリウム濃度"), "medicine assessment asks for application");
  assert(!assessmentText.includes("架空ケースで、最初に行う推論として最も適切なのはどれですか？"), "generic pressure-test question is absent");
  await rendererApp.click("button[aria-label=\"進捗\"]", "open Japanese medicine progress");
  await rendererApp.waitForSelector("#progress-heading", "Japanese medicine progress panel");
  const medicineProgress = await rendererApp.text();
  for (const objective of [
    "細胞内液・間質液・血漿の位置関係を説明し",
    "水分喪失とナトリウムを含む細胞外液喪失を区別し",
    "病歴、身体所見、バイタルサイン",
    "体液状態について妥当な推論と未確定事項",
  ]) {
    assert(medicineProgress.includes(objective), `progress shows Japanese objective: ${objective}`);
  }
  assert(!medicineProgress.includes("Explain intake and output"), "progress omits English legacy objectives");
  assert(!medicineProgress.includes("Objective"), "Japanese progress panel omits English objective labels");
  await rendererApp.click("button[aria-label=\"目次を表示\"]", "return from medicine progress");
  await rendererApp.waitForSelector("#lesson-heading", "medicine curriculum after progress");

  await openFixture("確率を条件で読み解く");
  await rendererApp.clickText("条件付き確率を表で読む", "open math resource");
  await rendererApp.waitForSelector(".math-inline .katex", "inline math output");
  await rendererApp.waitForSelector(".katex-display .katex", "display math output");
  await rendererApp.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await rendererApp.evaluate('document.querySelector(".display-settings").open = true');
  await rendererApp.clickText("200%", "enlarge math view");
  assert(
    await rendererApp.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"),
    "math resource fits a 390px viewport at 200% text size",
  );
  await rendererApp.send("Emulation.clearDeviceMetricsOverride");
  await rendererApp.clickText("目次へ", "return to mathematics outline");

  await rendererApp.evaluate("window.__navigationPerfStart = performance.now()");
  await rendererApp.click('button[aria-label="学習ルートを見る"]', "open Route in the exam-oriented mathematics Package");
  await rendererApp.waitForSelector("#route-heading", "mathematics Route");
  const mathRouteMs = await rendererApp.evaluate("performance.now() - window.__navigationPerfStart");
  assert(await rendererApp.evaluate('document.querySelectorAll(".route-step").length === 4'), "the exam-oriented math Route contains its four authored objectives");
  await rendererApp.click(".route-step:first-child .route-step-button", "select the first exam-study objective before opening its Atlas");
  await rendererApp.click(".route-atlas-link", "open local Atlas in the mathematics Package");
  await rendererApp.waitForSelector("#atlas-heading", "mathematics Package-local Atlas");
  await rendererApp.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "確率と標本空間"', "math probability Concept as the Atlas center");
  assert(await rendererApp.evaluate('document.querySelector(".atlas-dependent-lane")?.innerText.includes("事象と集合演算") === true'), "math Atlas shows its explicit probability-to-events dependency");
  await rendererApp.setField('.atlas-search input', "心不全");
  await rendererApp.waitFor('() => document.querySelector(".atlas-search-results")?.innerText.includes("一致するテーマはありません") === true', "medical Concept names do not leak into the math Atlas");
  await rendererApp.evaluate("window.__navigationPerfStart = performance.now()");
  await rendererApp.setField('.atlas-search input', "ベイズ");
  await rendererApp.waitForSelector(".atlas-search-results button", "Bayes Concept search in the math Atlas");
  await rendererApp.waitFor('() => document.querySelector(".atlas-search-results")?.innerText.includes("ベイズの定理と事前確率") === true', "Bayes result is package-local");
  const mathSearchMs = await rendererApp.evaluate("performance.now() - window.__navigationPerfStart");
  await rendererApp.click(".atlas-search-results button", "recenter the math Atlas on Bayes");
  await rendererApp.waitFor('() => document.querySelector(".atlas-focus h3")?.textContent === "ベイズの定理と事前確率"', "Bayes becomes the Atlas center");
  await rendererApp.clickText("Lessonへ戻る", "return to the math Lesson after Atlas exploration");
  await rendererApp.waitForSelector("#lesson-heading", "math Lesson after Atlas exploration");
  pass("exam-oriented mathematics Package uses the same Route/Atlas flow", `Route ${mathRouteMs.toFixed(0)} ms; search ${mathSearchMs.toFixed(0)} ms; four-Concept chain remains package-local`);

  await openFixture("旅行で使う英語：道をたずね、案内する");
  await rendererApp.clickText("聞き取れた語をつなぎ、聞き返す", "open language resource");
  await rendererApp.waitForSelector(".reader .markdown", "language resource body");
  const languageText = await rendererApp.text();
  assert(languageText.includes("ˈsteɪ.ʃən"), "IPA text renders from language package");
  assert(languageText.includes("左と言いましたか"), "Japanese content remains unchanged");
  await rendererApp.clickText("目次へ", "return to language outline");

  await openFixture("プログラムを読む・エラーを追う・変更を確かめる");
  await rendererApp.clickText("コードを実行せずに一行ずつ読む", "open programming resource");
  await rendererApp.waitForSelector('.code-frame [class*="hljs-"]', "highlighted code output");
  const inertMarkupCheck = await rendererApp.evaluate(`(() => {
    const body = document.querySelector(".markdown");
    const links = [...(body?.querySelectorAll("a") ?? [])];
    const labels = links.map((link) => link.textContent.trim());
    return {
      unsafeHref: links.some((link) => link.hasAttribute("href") && !/^(https?:|mailto:)/i.test(link.getAttribute("href") ?? "")),
      rejectedLabelsAreText: body?.innerText.includes("端末を開くリンク") === true && body.innerText.includes("Package外を指す相対リンク") && !links.some((link) => link.textContent.includes("端末を開くリンク") || link.textContent.includes("Package外を指す相対リンク")),
      imageInjected: body?.querySelector("img") !== null,
      fallbackText: body?.innerText.includes("図が読み込めないときの説明") === true,
      rawHtmlIsText: body?.innerText.includes("<img src=x onerror=") === true,
      externalLink: labels.includes("Python公式Tutorial"),
      localReferenceHasNoNavigationTarget: body?.innerText.includes("Package外を指す相対リンク") === true && !links.some((link) => link.textContent.includes("Package外を指す相対リンク")),
    };
  })()`);
  assert(!inertMarkupCheck.unsafeHref, "unsafe package links never become clickable anchors");
  assert(inertMarkupCheck.rejectedLabelsAreText, "unsafe link labels remain readable plain text");
  assert(!inertMarkupCheck.imageInjected, "package images never load or inject an image element");
  assert(inertMarkupCheck.fallbackText, "image alt text remains visible as a fallback");
  assert(inertMarkupCheck.rawHtmlIsText, "raw HTML remains visible as inert text");
  assert(inertMarkupCheck.externalLink, "Core-classified external links render as anchors");
  assert(inertMarkupCheck.localReferenceHasNoNavigationTarget, "package-relative references render without an unsafe navigation target");
  await rendererApp.evaluate(`(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: async (text) => { window.__osmiumCopied = text; } },
    });
  })()`);
  await rendererApp.click(".code-copy", "copy code button");
  await rendererApp.waitFor(
    '() => typeof window.__osmiumCopied === "string" && window.__osmiumCopied.includes("format_label(base, count)")',
    "copy button writes the source code to the clipboard",
  );
  pass("copy code button copies inert source text");
  await rendererApp.send("Emulation.setDeviceMetricsOverride", {
    width: 390,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await rendererApp.evaluate('document.querySelector(".display-settings").open = true');
  await rendererApp.clickText("200%", "enlarge programming code view");
  const codeCheck = await rendererApp.evaluate(`(() => ({
    image: document.querySelector(".markdown img") !== null,
    source: [...document.querySelectorAll(".code-frame code")].map((code) => code.textContent ?? "").join(String.fromCharCode(10)),
    frameOverflows: [...document.querySelectorAll(".code-frame .md-pre")].some((frame) => frame.scrollWidth > frame.clientWidth),
    pageFits: document.documentElement.scrollWidth <= window.innerWidth + 1,
  }))()`);
  assert(!codeCheck.image, "package code does not create an HTML image element");
  assert(codeCheck.source.includes("format_label(base, count)"), "code remains displayed as inert text");
  assert(codeCheck.source.includes("normalize_record"), "long code line is preserved");
  assert(codeCheck.pageFits, "programming resource fits a 390px viewport at 200% text size");
  assert(codeCheck.frameOverflows, "long code line scrolls within its code frame");
  await rendererApp.send("Emulation.clearDeviceMetricsOverride");
  await rendererApp.stop();
  pass("cross-domain renderer session shut down");

  const installedBeforeRemoval = cli(["packages", "--json"]).payload?.data ?? [];
  assert(installedBeforeRemoval.length > 0, "all integration packages are present before uninstall");
  for (const packageInfo of installedBeforeRemoval) {
    const removed = cli([
      "uninstall",
      packageInfo.package_id,
      "--package-version",
      packageInfo.package_version,
      "--json",
    ]);
    assert(removed.payload?.ok === true, "package uninstall preserves the local event store", packageInfo.package_id);
  }
  const historyOnlyApp = new App();
  await historyOnlyApp.start();
  await historyOnlyApp.click('button[aria-label="履歴"]', "open history after all package payloads are uninstalled");
  await historyOnlyApp.waitForSelector("#history-heading", "global history without an installed package");
  await historyOnlyApp.waitFor(
    '() => document.querySelectorAll(".history-list > li").length > 0',
    "orphaned package events remain visible",
  );
  const orphanHistory = await historyOnlyApp.text();
  assert(orphanHistory.includes("1 + 1 はいくつ？"), "assessment snapshot keeps the question readable after uninstall");
  assert(orphanHistory.includes("org.example/arithmetic@0.1.0"), "orphan history identifies its Package and version");
  await historyOnlyApp.stop();
  assert(
    (cli(["packages", "--json"]).payload?.data ?? []).length === 0,
    "all package payloads are removed while the event store remains",
  );
  pass("Desktop presents retained history after every Package payload is removed");
  pass("the history-only session shut down");

  // P2: exercise a real Library-button uninstall, restart, orphan history,
  // same-distribution reinstall, and digest-scoped progress restoration.
  const cardioId = "org.example/cardiovascular-atlas-validation";
  assert(cli(["install", cardiovascularArchive, "--json"]).payload?.ok === true, "the cardiovascular Package is installed for lifecycle validation");
  const cardioBefore = (cli(["packages", "--json"]).payload?.data ?? []).find((item) => item.package_id === cardioId && item.package_version === "0.1.0");
  assert(Boolean(cardioBefore?.digest), "the installed Package has a payload digest before UI deletion");
  const cardioPayload = join(home, "library", cardioBefore.digest);
  assert(existsSync(cardioPayload), "the installed payload directory exists before UI deletion", cardioPayload);
  let lifecycleApp = new App();
  await lifecycleApp.start();
  await lifecycleApp.click(`[data-package-id="${cardioId}"][data-package-version="0.1.0"]`, "open the cardiovascular Package from Library");
  await lifecycleApp.waitForSelector("#lesson-heading", "cardiovascular Package opens before deletion");
  await lifecycleApp.click(".curriculum-section:nth-of-type(2) summary", "open the section with retained preload progress");
  assert((await lifecycleApp.text()).includes("2 回の回答"), "same-digest reinstall restores observed progress before deletion");
  await lifecycleApp.click(".app-nav button:first-child", "return to Library for the destructive action");
  await lifecycleApp.evaluate("window.osmiumE2eConfirmRemoval = true");
  await lifecycleApp.click(`li.library-card:has([data-package-id="${cardioId}"][data-package-version="0.1.0"]) button.package-remove`, "delete this Package through the real Library UI");
  await lifecycleApp.waitFor(`() => document.querySelector('li.library-card:has([data-package-id=${JSON.stringify(cardioId)}][data-package-version="0.1.0"])') === null`, "the Package card disappears after successful uninstall");
  assert(await lifecycleApp.evaluate(`document.querySelector("#lesson-heading") === null && document.querySelector("#atlas-heading") === null && document.querySelector('.app-nav button[aria-label="目次を表示"]')?.disabled === true`), "deleting the selected Package clears learner, Route, and Atlas selection state");
  await lifecycleApp.waitFor('() => document.querySelector("[role=status]")?.innerText.includes("学習履歴は保持されています") === true', "UI confirms uninstall and retained learning history");
  assert(!existsSync(cardioPayload), "Library uninstall removes the on-disk Package payload directory", cardioPayload);
  await lifecycleApp.stop();
  assert(!(cli(["packages", "--json"]).payload?.data ?? []).some((item) => item.package_id === cardioId && item.package_version === "0.1.0"), "Runtime no longer lists the uninstalled Package");
  const cardioEventsAfterRemove = cli(["history", cardioId, "--package-version", "0.1.0", "--json"]).payload?.data ?? [];
  assert(cardioEventsAfterRemove.length === 2, "both append-only assessment events remain after deleting the Package", `${cardioEventsAfterRemove.length} retained events`);
  lifecycleApp = new App();
  await lifecycleApp.start();
  await lifecycleApp.waitFor('() => document.querySelector("#packages-heading") !== null', "Library after full application restart");
  assert(await lifecycleApp.evaluate(`document.querySelector('[data-package-id=${JSON.stringify(cardioId)}][data-package-version="0.1.0"]') === null`), "deleted Package stays absent from Library after restart");
  await lifecycleApp.click('button[aria-label="履歴"]', "open retained History after Package uninstall and restart");
  await lifecycleApp.waitForSelector("#history-heading", "orphaned cardiovascular learning history");
  assert((await lifecycleApp.text()).includes("前負荷の概念に最も近い説明はどれですか？"), "assessment snapshot remains readable without the Package payload");
  assert((await lifecycleApp.text()).includes(`${cardioId}@0.1.0`), "orphan history retains Package identity and version");
  await lifecycleApp.stop();
  assert(cli(["install", cardiovascularArchive, "--json"]).payload?.ok === true, "the same distribution can be reinstalled after UI uninstall");
  lifecycleApp = new App();
  await lifecycleApp.start();
  await lifecycleApp.click(`[data-package-id="${cardioId}"][data-package-version="0.1.0"]`, "reopen the reinstalled same-digest Package");
  await lifecycleApp.waitForSelector("#lesson-heading", "same Package after reinstall");
  await lifecycleApp.click(".curriculum-section:nth-of-type(2) summary", "reopen the progress-bearing section after reinstall");
  assert((await lifecycleApp.text()).includes("2 回の回答"), "the exact same digest restores its prior observed progress");
  await lifecycleApp.stop();
  pass("P2 Library lifecycle removes payload, survives restart, retains History, and restores progress after same-digest reinstall");

  console.log(`\n${steps.length} checks passed.`);
}

main().catch(async (error) => {
  for (const app of runningApps) await app.stop();
  console.error(`\nEND-TO-END FAILURE: ${error?.message ?? String(error)}`);
  process.exitCode = 1;
});

// A rejected socket or fetch must be reported, never silently end the run.
process.on("unhandledRejection", (reason) => {
  console.error(`\nUNHANDLED REJECTION: ${reason?.message ?? String(reason)}`);
  process.exitCode = 1;
});
process.on("uncaughtException", (error) => {
  console.error(`\nUNCAUGHT EXCEPTION: ${error?.message ?? String(error)}`);
  process.exitCode = 1;
});
