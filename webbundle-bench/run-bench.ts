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

const port = args.port ?? 8080;

const browser = await puppeteer.launch({ executablePath });

for (const name of ["unbundled", "webbundle"]) {
  await run(`${name}`, browser, `http://localhost:${port}/out/${name}.html`);
}

browser.close();

async function run(name: string, browser: Browser, url: string) {
  console.log(`running ${name} - ${url}`);
  const page = await browser.newPage();
  await page.goto(url, { waitUntil: "networkidle0" });
  const ele = await page.$("#log");
  const results = JSON.parse(
    await page.evaluate((elm) => elm.textContent, ele),
  );
  console.log(
    name + ": " + (results.importEnd - results.navigationResponseStart),
  );
}
