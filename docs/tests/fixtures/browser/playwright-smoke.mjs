import { chromium } from "playwright";

const baseUrl = process.argv[2] || "http://127.0.0.1:4173/";
const viewport = { width: 1280, height: 800 };

async function runCase(name, fn) {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport });
  const started = Date.now();
  try {
    await page.goto(`${baseUrl}?scenario=${name}&reset=1`, { waitUntil: "domcontentloaded" });
    await fn(page);
    const evidence = await page.locator("#evidence").innerText();
    const wallTimeMs = Date.now() - started;
    console.log(`${name}: pass ${wallTimeMs}ms | ${evidence}`);
  } finally {
    await browser.close();
  }
}

async function expectEvidence(page, text) {
  await page.locator("#evidence").filter({ hasText: text }).waitFor();
}

await runCase("browser-button-state", async (page) => {
  await page.getByRole("button", { name: "Activate Reactor" }).click();
  await expectEvidence(page, "Status: Flux stabilized");
});

await runCase("browser-form-validation", async (page) => {
  await page.getByLabel("Name").fill("Ada Lovelace");
  await page.getByLabel("Email").fill("ada@example.test");
  await page.getByLabel("Message").fill("Benchmark message accepted.");
  await page.getByRole("button", { name: "Submit" }).click();
  await expectEvidence(page, "Submission received for Ada Lovelace");
});

await runCase("browser-multi-step-wizard", async (page) => {
  await page.getByLabel("Stable Route").check();
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByLabel("Operator").fill("Mina Patel");
  await page.getByLabel("Code").fill("Q4-17");
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Confirm" }).click();
  await expectEvidence(page, "Stable Route | Mina Patel | Q4-17");
});

await runCase("browser-table-filter", async (page) => {
  await page.getByLabel("Filter inventory").fill("cobalt");
  await expectEvidence(page, "Only visible row: Cobalt Ridge");
});

await runCase("browser-scroll-target", async (page) => {
  await page.locator("#archive-node-42").scrollIntoViewIfNeeded();
  await page.getByRole("button", { name: "Select Archive Node 42" }).click();
  await expectEvidence(page, "Selected Archive Node 42");
});

await runCase("desktop-coordinate-stability", async (page) => {
  const button = page.getByRole("button", { name: "Click Target" });
  for (let i = 0; i < 5; i += 1) {
    await button.click();
  }
  await expectEvidence(page, "Successful clicks: 5");
});
