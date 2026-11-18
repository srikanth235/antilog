// node --test deploy/tofu/aws/functions
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("./spa-fallback.js", import.meta.url), "utf8");
const handler = new Function(`${source}; return handler;`)();
const uri = (path) => handler({ request: { uri: path } }).uri;

test("a screen of the app is answered with the app", () => {
  for (const path of ["/", "/projects", "/experiments/01abc", "/auth/github/callback", "/settings/members/"]) {
    assert.equal(uri(path), "/index.html", path);
  }
});

test("a file keeps its path, so a missing one is a real 404", () => {
  for (const path of ["/index.html", "/assets/app-3f2.js", "/favicon.svg", "/a.b/c.css"]) {
    assert.equal(uri(path), path, path);
  }
});
