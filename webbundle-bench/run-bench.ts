// Usage: deno run --allow-all run-bench.ts --browser ~/src/chrome1/src/out/Default/chrome [--port xxxx]

import {
  Browser,
  default as puppeteer,
} from "https://deno.land/x/puppeteer@14.1.1/mod.ts";
import { parse } from "https://deno.land/std@0.141.0/flags/mod.ts";

const args = parse(Deno.args);

const executablePath = args.browser;
if (!executablePath) {
  console.error("--browser [path] must be specified");
  Deno.exit(1);
}

const launch_options = {
  executablePath,
  args: (args._ ?? []) as string[],
};

// console.log(`browser launch options: ${JSON.stringify(launch_options)}`);
const browser = await puppeteer.launch(launch_options);

const port = args.port ?? 8080;

async function run(name: string, browser: Browser, url: string) {
  console.log(`running ${name} - ${url}`);
  const page = await browser.newPage();
  // TODO: Support trace.
  // await page.tracing.start({ path: `${name}-trace.json` });
  await page.goto(url, { waitUntil: "networkidle0" });
  // await page.tracing.stop();
  const ele = await page.$("#result");
  const results = JSON.parse(
    await page.evaluate((elm) => elm.textContent, ele),
  );
  console.log(
    name + ": " + (results.importEnd - results.navigationResponseStart),
  );
}

for (const name of ["unbundled", "webbundle"]) {
  await run(`${name}`, browser, `http://localhost:${port}/${name}.html`);
}

await browser.close();

// [2022-11-16 Wed] deno doesn't finish. Call exit explicitly.
Deno.exit(0);
