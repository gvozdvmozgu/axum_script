import { assert, assertEquals } from "jsr:@std/assert@1";

Deno.test("Text from DB", async () => {
  const resp = await fetch("http://localhost:4000/db-txt");
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
