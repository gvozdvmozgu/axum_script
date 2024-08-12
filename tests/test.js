import { assert, assertEquals } from "jsr:@std/assert@1";

Deno.test("Text from DB", async () => {
  const resp = await fetch("http://localhost:4000/db-txt");
  const txt = await resp.text();
  assertEquals(txt, "hello from the function foo 1");
});

//const jsonData = await resp.json();
//console.log(jsonData);
