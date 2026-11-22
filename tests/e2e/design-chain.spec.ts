// E2E Playwright — chaîne de conception de l'Atelier (VD-22).
//
// Prérequis : build statique (`.\scripts\build.cmd`), serveur
// (`node scripts\serve.mjs 4173`), puis `npx playwright test tests/e2e`.
// Ce fichier n'est pas exécuté par `cargo test` ; il dérive de la spec, pas du
// code livré. Les sélecteurs sont volontairement sémantiques (texte visible).

import { test, expect } from "@playwright/test";

const BASE = process.env.ARCHIFIER_URL ?? "http://127.0.0.1:4173";

test.beforeEach(async ({ page }) => {
  await page.goto(BASE + "/index.html");
  await page.evaluate(() => localStorage.removeItem("archifier:last_view"));
  await page.goto(BASE + "/#/atelier");
});

test("l'accueil ouvre le portefeuille de produits, jamais un inventaire", async ({ page }) => {
  await page.goto(BASE + "/#/");
  await expect(page.getByRole("heading", { name: "Produits" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Carte" })).toBeVisible();
  await expect(page.getByText("Nouveau domaine")).toHaveCount(0);
});

test("chaîne de conception : déposer, nommer, relier, formaliser, publier", async ({ page }) => {
  await page.getByRole("button", { name: /Applicatif d'un domaine/ }).click();
  await page.getByRole("button", { name: "Créer cette vue" }).click();
  await expect(page.getByText(/Enregistré · révision/)).toBeVisible();

  // Déposer un composant (un geste).
  await page.locator(".palette-entry", { hasText: "Composant applicatif" }).first().click();
  // Nommer sur place (inspecteur, non bloquant).
  const name = page.locator(".inspector input").first();
  await name.fill("Pricing");
  await name.press("Enter");
  // Formaliser : le type et le domaine sont proposés.
  await page.getByRole("button", { name: "Formaliser" }).click();
  await expect(page.getByText("VALIDÉ")).toBeVisible();

  // Second composant, formalisé.
  await page.locator(".palette-entry", { hasText: "Service applicatif" }).first().click();
  await page.locator(".inspector input").first().fill("Encaissement");
  await page.locator(".inspector input").first().press("Enter");
  await page.getByRole("button", { name: "Formaliser" }).click();

  // Relier : uniquement les relations autorisées.
  await page.getByRole("button", { name: "Relation", exact: true }).click();
  await page.locator(".node", { hasText: "Pricing" }).click();
  await page.locator(".node", { hasText: "Encaissement" }).click();
  await expect(page.getByRole("region", { name: "Relier" })).toBeVisible();
  await page.locator(".relation-bar select").selectOption("serving");
  await page.getByRole("button", { name: "Relier", exact: true }).click();
  await expect(page.locator(".edge")).toHaveCount(1);

  // Frontière et port.
  await page.getByRole("button", { name: "Zone de domaine" }).click();
  await page.locator(".zone-domain").last().click();
  await page.locator(".inspector select").selectOption({ label: "Payment API" });
  await page.getByRole("button", { name: "Rattacher une interface" }).click();
  await expect(page.locator(".port")).toHaveCount(1);

  // Curseur temporel : état lisible sans fiche.
  await page.locator(".temporal-input").fill("2027-08-01");
  await expect(page.locator(".node-state")).toContainText("Expirée");

  // Publier (après avoir retiré les croquis restants).
  for (const draft of await page.locator(".draft").all()) {
    await draft.click();
    await page.getByRole("button", { name: "Retirer de la vue" }).click();
  }
  await page.getByRole("button", { name: "Publier" }).click();
  await expect(page.getByText("Publié")).toBeVisible();

  // Lien partagé : posture lecture, cadrage et curseur restitués.
  await page.getByRole("button", { name: "Copier le lien" }).click();
  const shared = await page.locator(".banner.info input").inputValue();
  await page.goto(shared);
  await expect(page.getByRole("link", { name: "Modifier" })).toBeVisible();
  await expect(page.locator(".temporal-input")).toHaveValue("2027-08-01");
  await expect(page.locator(".palette")).toHaveCount(0);

  // Bibliothèque et Contrôle : espaces secondaires.
  await page.getByRole("link", { name: "Bibliothèque" }).click();
  await expect(page.locator(".cover").first()).toBeVisible();
  await page.getByRole("link", { name: "Contrôle" }).click();
  await expect(page.getByRole("link", { name: "Inventaire" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Créer/ })).toHaveCount(0);
});

test("DSG-001 : formaliser sans type est refusé et laisse le croquis", async ({ page }) => {
  await page.getByRole("button", { name: "Créer cette vue" }).click();
  await page.locator(".palette-entry", { hasText: "Composant applicatif" }).first().click();
  await page.locator(".inspector select").first().selectOption("");
  await page.getByRole("button", { name: "Formaliser" }).click();
  await expect(page.locator(".banner.error")).toContainText("DSG-001");
  await expect(page.locator(".draft")).toHaveCount(3); // 2 du modèle + 1 posé
});

test("DSG-003 : publier avec des croquis est refusé", async ({ page }) => {
  await page.getByRole("button", { name: "Créer cette vue" }).click();
  await page.getByRole("button", { name: "Publier" }).click();
  await expect(page.locator(".banner.error")).toContainText("DSG-003");
});

test("un lien arch:// de relation n'est plus cassé", async ({ page }) => {
  await page.goto(BASE + "/#/library/stories");
  const first = page.locator(".link-list a").first();
  await first.click();
  await expect(page.getByRole("heading")).toBeVisible();
  await expect(page.getByText("Page introuvable")).toHaveCount(0);
});
