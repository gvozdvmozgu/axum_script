//import { op_route } from "ext:core/ops";

console.log("core");
route("foo", async () => {
  console.log("THIS IS INSIDE the FUNCTION");
  return "hello from the function";
});
