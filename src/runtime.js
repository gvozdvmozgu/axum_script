((globalThis) => {
  const core = Deno.core;

  function argsToMessage(...args) {
    return args
      .map((arg) => (typeof arg === "string" ? arg : JSON.stringify(arg)))
      .join(" ");
  }

  globalThis.console = {
    log: (...args) => {
      core.print(`${argsToMessage(...args)}\n`, false);
    },
    error: (...args) => {
      core.print(`[err]: ${argsToMessage(...args)}\n`, true);
    },
  };

  globalThis.route = Deno.core.ops.op_route;
  globalThis.query = Deno.core.ops.op_query;
  globalThis.sleep = Deno.core.ops.op_sleep;
  globalThis.createCache = async (f) => {
    Deno.core.ops.op_create_cache(f);
    await Deno.core.ops.op_flush_cache();
  };
})(globalThis);
