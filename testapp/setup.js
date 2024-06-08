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
  return "hello from the function in  main";
});
