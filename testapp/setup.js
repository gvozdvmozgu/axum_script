//import { op_route } from "ext:core/ops";

Deno.core.print("core"); //, Object.keys(Deno.core).join(""));
Deno.core.print(JSON.stringify(Deno)); //, Object.keys(Deno.core).join(""));
Deno.core.ops.op_route("foo", () => {});
