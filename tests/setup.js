import {} from "./other.js";

await execute(`create table if not exists names (
   id INTEGER PRIMARY KEY,
   name TEXT NOT NULL
);`);

await createCache(async () => {
  console.log("creating cache");
  const name_rows = await query("select name from names");
  const c = { akey: 1, bkey: 2, names: name_rows.map((row) => row.name) };
  console.log("new cache", c);

  return c;
});

route("/db-txt", async () => {
  const n = await query("select 1 as mynum");
  return `hello from the function foo ${n[0].mynum}`;
});

route("/db-json", async () => {
  const n = await query("select 1 as mynum");
  return { json: n };
});

route("/teapot", async () => {
  return { html: "short and stout", status: 418 };
});

route("/get-cache", async () => {
  return {
    json: {
      all: getCache(),
      akey: getCache("akey"),
      list: getCache(["akey"]),
      sum: getCache((c) => c.akey + c.bkey),
    },
  };
});

route("/baz/:id", async ({ id }) => {
  return `hello from the baz with arg ${id}`;
});

route("/insert-name/:name", async ({ name }) => {
  await execute(`insert into names(name) values ('${name}');`);
  await flushCache();
  return `OK`;
});

// to test for multithreading: autocannon -c 10 -d 5 -p 10 http://127.0.0.1:4000/sleep
route("/sleep", async () => {
  await sleep(100);
  return "hello from sleep";
});
