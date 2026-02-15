import fp from "fastify-plugin";

export default fp(async (fastify) => {
  const config = loadConfig();
  fastify.decorate("myPlugin", {
    getValue: () => fetchValue(),
  });
});

function loadConfig() {
  return { key: "value" };
}

function fetchValue() {
  return 42;
}
