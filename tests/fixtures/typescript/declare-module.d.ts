declare module "fastify" {
  interface FastifyInstance {
    yisusClient: YisusClient;
    sessionStore: SessionStore;
  }

  interface FastifyRequest {
    userId: string;
  }
}

declare function globalHelper(x: number): string;
