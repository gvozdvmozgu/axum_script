import { assert, assertEquals } from "jsr:@std/assert@1";

Deno.test("Text from DB", async () => {
  const resp = await fetch("http://localhost:4000/db-txt");
  assertEquals(resp.headers.get("content-type"), "text/html; charset=utf-8");
  assertEquals(resp.status, 200);

  const txt = await resp.text();
  assertEquals(txt, "hello from the function foo 1");
});

Deno.test("JSON from DB", async () => {
  const resp = await fetch("http://localhost:4000/db-json");
  assertEquals(resp.headers.get("content-type"), "application/json");

  const j = await resp.json();

  assert(Array.isArray(j));
  assertEquals(j[0].mynum, 1);
});

Deno.test("Status code", async () => {
  const resp = await fetch("http://localhost:4000/teapot");

  assertEquals(resp.headers.get("content-type"), "text/html; charset=utf-8");
  assertEquals(resp.status, 418);
  const txt = await resp.text();

  assertEquals(txt, "short and stout");
});

Deno.test("Get cache", async () => {
  const resp = await fetch("http://localhost:4000/get-cache");
  assertEquals(resp.headers.get("content-type"), "application/json");
  assertEquals(resp.status, 200);

  const c = await resp.json();

  assertEquals(c.sum, 3);
  assertEquals(c.akey, 1);
  assertEquals(c.list.akey, 1);
});

Deno.test("Query string", async () => {
  const resp = await fetch("http://localhost:4000/baz/1");

  const txt = await resp.text();
  assertEquals(txt, "hello from the baz with arg 1");
});

Deno.test("Import", async () => {
  const resp = await fetch("http://localhost:4000/other");

  const txt = await resp.text();
  assertEquals(txt, "hello from import");
});
