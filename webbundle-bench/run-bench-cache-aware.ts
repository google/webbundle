// Usage: deno run --allow-all run-bench.ts --browser ~/src/chrome1/src/out/Default/chrome [--port xxxx]

import {
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

const port = args.port ?? 8080;

async function run(
  name: string,
  url1: string,
  url2: string,
) {
  console.log(`running cache-aware: cache_hit: ${name}`);

  // Launch new browser so that it doesn't have any cache.
  const browser = await puppeteer.launch(launch_options);

  {
    // 1st visit
    const page = await browser.newPage();

    // Connect to Chrome DevTools
    const client = await page.target().createCDPSession();

    // Set throttling property
    await client.send("Network.emulateNetworkConditions", {
      "offline": false,
      "downloadThroughput": 100 * 1000,
      "uploadThroughput": 100 * 1000,
      "latency": 50,
    });

    await page.goto(url1, { waitUntil: "networkidle0" });
    const ele = await page.$("#result");
    const results = JSON.parse(
      await page.evaluate((elm) => elm.textContent, ele),
    );
    console.log(
      name + ": 1st: " + (results.importEnd - results.navigationResponseStart),
    );
  }

  {
    // 2nd visit
    const page = await browser.newPage();

    // Connect to Chrome DevTools
    const client = await page.target().createCDPSession();

    // Set throttling property
    await client.send("Network.emulateNetworkConditions", {
      "offline": false,
      "downloadThroughput": 10 * 1000,
      "uploadThroughput": 10 * 1000,
      "latency": 50,
    });

    await page.goto(url2, { waitUntil: "networkidle0" });
    const ele = await page.$("#result");
    const results = JSON.parse(
      await page.evaluate((elm) => elm.textContent, ele),
    );

    console.log(
      name + ": 2nd: " + (results.importEnd - results.navigationResponseStart),
    );
  }

  await browser.close();
}

for (const name of [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]) {
  await run(
    `${name}`,
    `http://localhost:${port}/webbundle-cache-aware-${name}-1st.html`,
    `http://localhost:${port}/webbundle-cache-aware-${name}-2nd.html`,
  );
}

// [2022-11-16 Wed] deno doesn't finish. Call exit explicitly.
Deno.exit(0);
