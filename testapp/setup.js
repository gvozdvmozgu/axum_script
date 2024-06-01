//import { op_route } from "ext:core/ops";

Deno.core.print("core");
Deno.core.ops.op_route("foo", () => {
  Deno.core.print("THIS IS INSIDE the FUNCTION");
  return "hello from the function";
});
