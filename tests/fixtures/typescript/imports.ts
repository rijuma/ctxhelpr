import { readFileSync, writeFileSync } from "fs";
import path from "path";
import type { Config } from "./simple";

export function loadConfig(filePath: string): Config {
  const raw = readFileSync(filePath, "utf-8");
  return JSON.parse(raw);
}

export function saveConfig(config: Config, filePath: string): void {
  const data = JSON.stringify(config, null, 2);
  writeFileSync(filePath, data, "utf-8");
}

export function resolveConfigPath(base: string, file: string): string {
  return path.join(base, file);
}
