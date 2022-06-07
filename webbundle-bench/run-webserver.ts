import { serve } from "https://deno.land/std@0.141.0/http/server.ts";
import { parse } from "https://deno.land/std@0.141.0/flags/mod.ts";
export { default as staticFiles } from "https://deno.land/x/static_files@1.1.6/mod.ts";

function setHeaders(headers: Headers, path: string, _stats?: Deno.FileInfo) {
  if (path.endsWith(".wbn")) {
    headers.set("Content-Type", "application/webbundle");
    headers.set("X-Content-Type-Options", "nosniff");
  }
}

function handler(req: Request): Promise<Response> {
  return staticFiles(".", { setHeaders })({
    request: req,
    respondWith: (r: Response) => r,
  });
}

const args = parse(Deno.args);

const port = args.port ?? 8080;
const hostname = args.hostname ?? "127.0.0.1";

await serve(handler, { port, hostname });
