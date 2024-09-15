//import { op_route } from "ext:core/ops";

await connectToDatabase("sqlite://sqlite.db");

await execute(`create table if not exists names (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL
);`);

console.log("db created");

await createCache(async () => {
  console.log("creating cache");
  return { akey: 1, bkey: 2 };
});

route("/mkdb", async () => {
  await execute(`create table if not exists names (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
  );`);

  return `hello from mkdb`;
});

route("/foo", async () => {
  const n = await query("select 1 as mynum");
  return `hello from the function foo ${n[0].mynum}`;
});

route("/", async () => {
  console.log("full cache", getCache());
  console.log("skey cache", getCache("akey"));
  console.log("skey cache list", getCache(["akey"]));
  console.log(
    "skey cache function",
    getCache((c) => c.akey + c.bkey)
  );
  return "hello from the function in  main";
});

route("/baz/:id", async ({ params: { id } }) => {
  return `hello from the baz with arg ${id}`;
});

// to test for multithreading: autocannon -c 10 -d 5 -p 10 http://127.0.0.1:4000/sleep
route("/sleep", async () => {
  await sleep(9000);
  return "hello from sleep";
});
