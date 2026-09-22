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
import { existsSync, mkdirSync, rmSync } from "node:fs";
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
  return { status: result.status, stdout: result.stdout, stderr: result.stderr, payload };
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

class App {
  constructor() {
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
      env: { ...process.env, OSMIUM_HOME: home, OSMIUM_DEBUG_PORT: String(options.port) },
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
        throw new Error(`the application exited with code ${this.exited}: ${this.stderr}`);
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
        this.consoleErrors.push(JSON.stringify(message.params.exceptionDetails));
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
      throw new Error(`page exception: ${JSON.stringify(response.result.exceptionDetails)}`);
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
    throw new Error(`timed out waiting for ${description}. Body was:\n${await this.text()}`);
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
    const clicked = await this.evaluate(`(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (element === null) return false;
      element.click();
      return true;
    })()`);
    if (clicked !== true) fail(`click ${description}`, `no element for ${selector}`);
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
    try {
      this.socket?.close();
    } catch {
      // The socket may already be gone.
    }
    if (this.child !== null) {
      // The webview keeps a child process tree, so kill the whole tree.
      spawnSync("taskkill", ["/PID", String(this.child.pid), "/T", "/F"], { stdio: "ignore" });
      this.child = null;
    }
    for (let attempt = 0; attempt < 60; attempt += 1) {
      await sleep(250);
      try {
        await fetch(`${debugUrl}/json/version`, { signal: AbortSignal.timeout(500) });
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
const GOLDEN_OBJECTIVE = "addition.basic";

async function main() {
  console.log("# Osmium desktop end-to-end acceptance");
  console.log(`data root: ${home}`);
  console.log(`application: ${appPath}`);

  if (!existsSync(appPath)) fail("application binary", `${appPath} does not exist`);
  rmSync(home, { recursive: true, force: true });
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
  assert(linted.payload?.ok === true, "CLI lint accepts the Golden source", `findings ${linted.payload?.data?.finding_count}`);

  const built = cli(["build", source, "--output", archive]);
  assert(built.payload?.ok === true, "CLI build produces a portable archive", built.payload?.data?.digest?.slice(0, 16) ?? "");

  const archiveValidated = cli(["validate", archive, "--json"]);
  assert(archiveValidated.payload?.ok === true, "the built archive validates");

  const installed = cli(["install", archive, "--json"]);
  assert(installed.payload?.ok === true, "CLI install adds the package to the library");

  const listed = cli(["packages", "--json"]);
  assert(
    Array.isArray(listed.payload?.data) && listed.payload.data.length === 1,
    "the library lists exactly one installed package",
  );

  // 2. Start the application and confirm it renders the installed package.
  const app = new App();
  await app.start();
  assert((await app.text()).includes("インストール済みパッケージ"), "the application shows the packages panel");
  await app.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(PACKAGE_ID)})`,
    "the installed package in the list",
  );
  pass("the installed package is listed in the UI", PACKAGE_ID);

  // 3. Open the package, then the curriculum and concept.
  await app.click(".package", "the installed package");
  await app.waitForSelector("#lesson-heading", "the lesson panel");
  const lessonText = await app.text();
  assert(lessonText.includes("足し算"), "the curriculum / concept view renders", "concept 足し算");
  assert(lessonText.includes(GOLDEN_OBJECTIVE), "the objective is listed", GOLDEN_OBJECTIVE);

  // 4. Read the Markdown resource.
  await app.click('button[aria-label^="教材を開く"]', "the lesson resource");
  await app.waitForSelector("#reader-heading", "the reader panel");
  await app.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(GOLDEN_MARKER)})`,
    "the Markdown body",
  );
  const readerText = await app.text();
  assert(readerText.includes(GOLDEN_MARKER), "the Markdown lesson body is readable", GOLDEN_MARKER);
  assert(readerText.includes("足し算"), "the lesson heading is rendered");

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

  await app.click('input[type="radio"][value="b"]', "the correct option");
  await app.click('button[type="submit"]', "the grade button");
  await app.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(GOLDEN_FEEDBACK)})`,
    "the graded feedback",
  );
  const graded = await app.text();
  assert(graded.includes("正解"), "deterministic grading reports the answer as correct");
  assert(graded.includes("org.osmium.exact.v1"), "the evaluation engine version is shown", "org.osmium.exact.v1");
  assert(graded.includes(GOLDEN_FEEDBACK), "the feedback is rendered from the package", GOLDEN_FEEDBACK);

  // 6. Progress and history reflect the saved event. Both panels are reachable
  //    from the graded item as well as from the outline.
  await app.click('button[aria-label="進捗を表示"]', "the progress button");
  await app.waitForSelector("#progress-heading", "the progress panel");
  const progressText = await app.text();
  assert(progressText.includes(GOLDEN_OBJECTIVE), "progress lists the measured objective", GOLDEN_OBJECTIVE);
  assert(progressText.includes("100%"), "progress counts one correct attempt", "100%");

  await app.click('button[aria-label="履歴を表示"]', "the history button");
  await app.waitForSelector("#history-heading", "the history panel");
  await app.waitFor('() => document.body.innerText.includes("addition.01")', "the learning event");
  pass("the learning event is recorded in the history");

  // 7. Offline proof: only local assets and the IPC channel were requested.
  const external = app.requests.filter(
    (url) =>
      !url.startsWith("http://tauri.localhost") &&
      !url.startsWith("ipc:") &&
      !url.startsWith("http://ipc.localhost") &&
      !url.startsWith("data:") &&
      !url.startsWith("blob:"),
  );
  assert(external.length === 0, "the application performs no network request", `${app.requests.length} requests, all local`);
  assert(app.consoleErrors.length === 0, "the webview logged no error", app.consoleErrors.join(" | "));

  // 8. Fully stop, then restart and confirm the state survived.
  await app.stop();
  pass("the application shut down completely");

  const afterShutdown = cli(["history", PACKAGE_ID, "--json"]);
  assert(
    Array.isArray(afterShutdown.payload?.data) && afterShutdown.payload.data.length === 1,
    "the event log holds one attempt after shutdown",
  );

  const restarted = new App();
  await restarted.start();
  await restarted.waitFor(
    `() => document.body.innerText.includes(${JSON.stringify(PACKAGE_ID)})`,
    "the installed package after restart",
  );
  pass("the installed package is still listed after restart", PACKAGE_ID);

  await restarted.click(".package", "the installed package after restart");
  await restarted.waitForSelector("#lesson-heading", "the lesson panel after restart");
  const restartedLesson = await restarted.text();
  assert(restartedLesson.includes("1/1 正答"), "progress is restored on the lesson view", "1/1 正答");

  await restarted.click('button[aria-label="進捗を表示"]', "the progress button after restart");
  await restarted.waitForSelector("#progress-heading", "the progress panel after restart");
  const restartedProgress = await restarted.text();
  assert(restartedProgress.includes(GOLDEN_OBJECTIVE), "the objective survives the restart");
  assert(restartedProgress.includes("100%"), "the projected accuracy survives the restart", "100%");

  await restarted.clickText("目次へ", "back to the outline after restart");
  await restarted.waitForSelector("#lesson-heading", "the outline after restart");
  await restarted.click('button[aria-label="履歴を表示"]', "the history button after restart");
  await restarted.waitForSelector("#history-heading", "the history panel after restart");
  await restarted.waitFor('() => document.body.innerText.includes("addition.01")', "the learning event after restart");
  pass("the learning event survives the restart");

  await restarted.stop();
  pass("the application shut down again");

  console.log(`\n${steps.length} checks passed.`);
}

main().catch((error) => {
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
