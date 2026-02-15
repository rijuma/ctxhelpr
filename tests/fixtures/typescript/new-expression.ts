class TokenManager {
  private tokens: Map<string, string>;

  constructor() {
    this.tokens = new Map();
  }

  getToken(key: string): string | undefined {
    return this.tokens.get(key);
  }
}

class TokenRefreshError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TokenRefreshError";
  }
}

function createManager(): TokenManager {
  const manager = new TokenManager();
  return manager;
}

function handleError(error: unknown): void {
  if (error instanceof TokenRefreshError) {
    console.log("refresh error");
  } else if (error instanceof Error) {
    console.log("generic error");
  }
}
