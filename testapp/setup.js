//import { op_route } from "ext:core/ops";

console.log("core");
route("/foo", async () => {
  return "hello from the function foo";
});

route("/", async () => {
  return "hello from the function in  main";
});
