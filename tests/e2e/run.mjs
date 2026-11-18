// Lance les E2E Playwright du build statique.
//
// Usage : node tests/e2e/run.mjs
// Prérequis : serveur sur ARCHIFIER_URL (défaut http://127.0.0.1:4173) et
// Playwright installé. Sans Playwright, le script explique la limite et sort
// avec le code 2 (ce n'est pas un échec produit).

import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";

const url = process.env.ARCHIFIER_URL ?? "http://127.0.0.1:4173";

async function reachable() {
  try {
    const response = await fetch(url + "/index.html");
    return response.ok;
  } catch {
    return false;
  }
}

if (!(await reachable())) {
  console.error(`Serveur injoignable : ${url}. Lancez node scripts/serve.mjs 4173.`);
  process.exit(1);
}

if (!existsSync("node_modules/@playwright/test")) {
  console.error(
    "Playwright n'est pas installé (npm bloqué par la politique PowerShell ?).\n" +
      "Installez-le hors politique d'exécution, puis relancez :\n" +
      "  npm install -D @playwright/test && npx playwright install chromium\n" +
      "En attendant, la chaîne manuelle est décrite dans tests/e2e/README.md."
  );
  process.exit(2);
}

const result = spawnSync(
  process.platform === "win32" ? "npx.cmd" : "npx",
  ["playwright", "test", "tests/e2e"],
  { stdio: "inherit", env: { ...process.env, ARCHIFIER_URL: url } }
);
process.exit(result.status ?? 1);
