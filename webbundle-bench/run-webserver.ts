import { serve } from "https://deno.land/std@0.163.0/http/server.ts";
import { parse } from "https://deno.land/std@0.163.0/flags/mod.ts";

import { default as staticFiles } from "https://deno.land/x/static_files@1.1.6/mod.ts";

function setHeaders(headers: Headers, path: string, _stats?: Deno.FileInfo) {
  if (path.endsWith(".wbn")) {
    headers.set("Content-Type", "application/webbundle");
    headers.set("X-Content-Type-Options", "nosniff");
    // Vary header is necessary to prevent disk cache.
    headers.set("Vary", "bundle-preload");
  }
}

function handler(req: Request): Promise<Response> {
  console.log(">> ", req.url);

  // TODO: Check headers for bundlepreload.
  if (req.url.endsWith(".wbn")) {
    console.log("* bundle");
    // console.log("req.headers", req.headers);

    const bundle_preload = req.headers.get("bundle-preload");
    if (bundle_preload) {
      if (args.verbose) {
        console.log(
          "bunle-preload header:",
          bundle_preload,
          bundle_preload.split(",").length,
        );
      }

      if (!bundle_preload.includes('"a0.mjs"')) {
        // This should be 2nd visit.
        console.log(">>> * 2nd visit");
        return staticFiles("cache-aware-2nd", { setHeaders })({
          request: req,
          respondWith: (r: Response) => r,
        });
      }
    }
  }

  return staticFiles(".", { setHeaders })({
    request: req,
    respondWith: (r: Response) => r,
  });
}

const args = parse(Deno.args);

console.log("args: %o", args);

const port = args.port ?? 8080;
const hostname = args.hostname ?? "127.0.0.1";

await serve(handler, { port, hostname });
