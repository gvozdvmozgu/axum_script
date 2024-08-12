//import { op_route } from "ext:core/ops";

console.log("core");

await createCache(async () => {
  console.log("creating cache");
  return { akey: 1, bkey: 2 };
});

route("/db-txt", async () => {
  const n = await query("select 1 as mynum");
  return `hello from the function foo ${n[0].mynum}`;
});

route("/db-json", async () => {
  const n = await query("select 1 as mynum");
  return { json: n };
});

route("/cache-full", async () => {
  console.log("full cache", getCache());
  console.log("skey cache", getCache("akey"));
  console.log("skey cache list", getCache(["akey"]));
  console.log(
    "skey cache function",
    getCache((c) => c.akey + c.bkey)
  );
  return getCache();
});

route("/baz/:id", async ({ id }) => {
  return `hello from the baz with arg ${id}`;
});

// to test for multithreading: autocannon -c 10 -d 5 -p 10 http://127.0.0.1:4000/sleep
route("/sleep", async () => {
  await sleep(100);
  return "hello from sleep";
});
