// Experimental benchmark using "deno bench" feature

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
console.log(`browser launch options: ${JSON.stringify(launch_options)}`);
const browser = await puppeteer.launch(launch_options);

const port = args.port ?? 8080;
for (const name of ["unbundled", "webbundle"]) {
  Deno.bench(name, async () => {
    await run(browser, `http://localhost:${port}/out/${name}.html`);
  });
}

// browser should not be closed while running Bench.
// TODO: No way to await benchmark's finish?
// browser.close();

async function run(browser: Browser, url: string) {
  const page = await browser.newPage();

  // Disable cache
  await page.setCacheEnabled(false);

  await page.goto(url);
  await page.waitForSelector("#result");
}
