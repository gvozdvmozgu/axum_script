//import { op_route } from "ext:core/ops";

Deno.core.print("core"); //, Object.keys(Deno.core).join(""));
Deno.core.op_route("foo", () => {});
core.ops.op_route("foo", () => {});
op_route("foo", () => {});
