//import { op_route } from "ext:core/ops";

console.log("core");

route("/foo", async () => {
  const n = await query("select 1 as mynum");
  return `hello from the function foo ${n[0].mynum}`;
});

route("/", async () => {
  return "hello from the function in  main";
});

route("/baz/:id", async ({ id }) => {
  return `hello from the baz with arg ${id}`;
});

// to test for multithreading: autocannon -c 10 -d 5 -p 10 http://127.0.0.1:4000/sleep
route("/sleep", async () => {
  await sleep(100000);
  return "hello from sleep";
});
