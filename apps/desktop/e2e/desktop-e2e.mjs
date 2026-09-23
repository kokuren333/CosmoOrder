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
import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const defaultRepo = resolve(here, "..", "..", "..");

function parseArgs(argv) {
  const options = { port: 9333, repo: defaultRepo, osmium: null };
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (key === "--home") options.home = value;
    else if (key === "--app") options.app = value;
    else if (key === "--repo") options.repo = value;
    else if (key === "--osmium") options.osmium = value;
    else if (key === "--port") options.port = Number(value);
    else throw new Error(`unknown argument: ${key}`);
  }
  if (options.home === undefined) throw new Error("--home is required");
  if (options.app === undefined) throw new Error("--app is required");
  options.osmium ??= join(options.repo, "target", "debug", "osmium.exe");
  return options;
}

const options = parseArgs(process.argv.slice(2));
const home = resolve(options.home);
const appPath = resolve(options.app);
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
    for (let attempt = 0; attempt < 60; attempt += 1) {
      await sleep(250);
      try {
        await fetch(`${debugUrl}/json/version`, {
          signal: AbortSignal.timeout(500),
        });
      } catch {
        return;
      }
    }
    throw new Error("the application did not shut down");
  }
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

  const listed = cli(["packages", "--json"]);
  assert(
    Array.isArray(listed.payload?.data) && listed.payload.data.length === 1,
    "the library lists exactly one installed package",
  );

  // 2. Start the application and confirm it renders the installed package.
  const app = new App();
  await app.start();
  assert(
    (await app.text()).includes("ライブラリ"),
    "the application shows the packages panel",
  );
  assert(
    await app.evaluate(
      'document.querySelector(".brand-mark svg path") !== null',
    ),
    "the header uses the custom Osmium crystal mark",
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
    await app.evaluate('(() => { const text = document.querySelector(".package-counts")?.innerText ?? ""; return ["Concepts", "Objectives", "Resources", "Assessments"].every((label) => text.includes(label)); })()'),
    "the library card shows all four package counts",
  );

  assert(
    !(await app.text()).includes(PACKAGE_ID),
    "library metadata is collapsed",
  );
  await app.click(".library-card summary", "library details");
  assert(
    (await app.text()).includes(PACKAGE_ID),
    "library details expose package metadata",
  );
  await app.click(".library-card summary", "close library details");
  await checkPresentation(app, "library");

  // 3. Open the package, then the curriculum and concept.
  await app.click(".package", "the installed package");
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
    ["CONCEPT", "OBJECTIVE", "RESOURCE", "ASSESSMENT"].every((label) => lessonText.includes(label)),
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

  await restarted.click(".package", "the installed package after restart");
  await restarted.waitForSelector(
    "#lesson-heading",
    "the lesson panel after restart",
  );
  const restartedLesson = await restarted.text();
  assert(
    restartedLesson.includes("1/2 正答"),
    "progress is restored on the lesson view",
    "1/2 正答",
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

  await openFixture("体液バランスと臨床的推論");
  await rendererApp.clickText("Fluid balance", "open medicine resource");
  await rendererApp.waitForSelector(".markdown table", "medicine table");
  assert(
    await rendererApp.evaluate('document.querySelector(".table-scroll") !== null'),
    "medicine resource table is in a scrollable wrapper",
  );
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

  await openFixture("Probability and conditional reasoning");
  await rendererApp.clickText("Conditional probability", "open math resource");
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

  await openFixture("Japanese and English: asking for directions");
  await rendererApp.clickText("Direction phrases", "open language resource");
  await rendererApp.waitForSelector(".reader .markdown", "language resource body");
  const languageText = await rendererApp.text();
  assert(languageText.includes("ˈsteɪʃən"), "IPA text renders from language package");
  assert(languageText.includes("左に曲がる"), "Japanese content remains unchanged");
  await rendererApp.clickText("目次へ", "return to language outline");

  await openFixture("Reading safe program output");
  await rendererApp.clickText("Inspecting code as text", "open programming resource");
  await rendererApp.waitForSelector('.code-frame [class*="hljs-"]', "highlighted code output");
  const inertMarkupCheck = await rendererApp.evaluate(`(() => {
    const body = document.querySelector(".markdown");
    const links = [...(body?.querySelectorAll("a") ?? [])];
    const labels = links.map((link) => link.textContent.trim());
    return {
      unsafeHref: links.some((link) => link.hasAttribute("href") && !/^(https?:|mailto:)/i.test(link.getAttribute("href") ?? "")),
      rejectedLabelsAreText: body?.innerText.includes("blocked script") === true && body.innerText.includes("blocked traversal") && !links.some((link) => link.textContent.includes("blocked script") || link.textContent.includes("blocked traversal")),
      imageInjected: body?.querySelector("img") !== null,
      fallbackText: body?.innerText.includes("diagram fallback text") === true,
      rawHtmlIsText: body?.innerText.includes('<img src="x" onerror="alert(1)">') === true,
      externalLink: labels.includes("External reference"),
      localReferenceHasNoNavigationTarget: links.some((link) => link.textContent.includes("Local package reference") && !link.hasAttribute("href")),
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
    '() => typeof window.__osmiumCopied === "string" && window.__osmiumCopied.includes("print(message)")',
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
    source: document.querySelector(".code-frame code")?.textContent ?? "",
    frameOverflows: (() => { const frame = document.querySelector(".code-frame .md-pre"); return frame !== null && frame.scrollWidth > frame.clientWidth; })(),
    pageFits: document.documentElement.scrollWidth <= window.innerWidth + 1,
  }))()`);
  assert(!codeCheck.image, "package code does not create an HTML image element");
  assert(codeCheck.source.includes('print(message)'), "code remains displayed as inert text");
  assert(codeCheck.source.includes("audit_metadata"), "long code line is preserved");
  assert(codeCheck.pageFits, "programming resource fits a 390px viewport at 200% text size");
  assert(codeCheck.frameOverflows, "long code line scrolls within its code frame");
  await rendererApp.send("Emulation.clearDeviceMetricsOverride");
  await rendererApp.stop();
  pass("cross-domain renderer session shut down");

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
