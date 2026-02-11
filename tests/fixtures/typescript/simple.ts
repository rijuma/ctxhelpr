/** Adds two numbers */
export function add(a: number, b: number): number {
  return a + b;
}

/** Multiplies two numbers */
function multiply(x: number, y: number): number {
  return x * y;
}

interface Config {
  port: number;
  host: string;
  debug?: boolean;
}

class Server {
  private config: Config;

  constructor(config: Config) {
    this.config = config;
  }

  /** Start the server */
  start(): void {
    listen(this.config.port);
  }

  stop(): void {
    shutdown();
  }
}

type Handler = (req: Request, res: Response) => void;

enum Status {
  Active,
  Inactive,
  Pending,
}

const DEFAULT_PORT: number = 3000;

const greet = (name: string): string => {
  return `Hello, ${name}`;
};
