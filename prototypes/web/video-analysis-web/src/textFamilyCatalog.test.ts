import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { packageDomainFor } from "./workspaceArchitecture";
import { textFamilyEntryHref, textFamilyTiers } from "./textFamilyCatalog";

const workspaceRoot = fileURLToPath(new URL("../../../..", import.meta.url));

describe("text family catalog", () => {
  it("maps every entry to an indexed text wrapper and existing recommended preset", () => {
    for (const entry of allEntries()) {
      const appPath = `${workspaceRoot}/packages/${entry.library}-app/src/App.tsx`;
      const serverPath = `${workspaceRoot}/crates/text/${entry.library}-server/Cargo.toml`;

      expect(packageDomainFor(`moritzbrantner-${entry.library}-server`, `crates/text/${entry.library}-server`)).toBe("text");
      expect(existsSync(serverPath), `${entry.library} server wrapper should exist`).toBe(true);
      expect(existsSync(appPath), `${entry.library} app should exist`).toBe(true);

      if (entry.presetId) {
        const appSource = readFileSync(appPath, "utf8");
        expect(extractIds(appSource), `${entry.library} should include preset ${entry.presetId}`).toContain(entry.presetId);
        expect(textFamilyEntryHref(entry)).toBe(`/wrappers/${entry.library}/?preset=${entry.presetId}`);
      }
    }
  });

  it("keeps text-retrieval only in the collapsed compatibility tier", () => {
    const retrievalTiers = textFamilyTiers.filter((tier) =>
      tier.entries.some((entry) => entry.library === "text-retrieval"),
    );

    expect(retrievalTiers).toHaveLength(1);
    expect(retrievalTiers[0]?.id).toBe("compatibility");
    expect(retrievalTiers[0]?.collapsedByDefault).toBe(true);
  });

  it("makes text-analysis the primary entry point", () => {
    const primaryTier = textFamilyTiers.find((tier) => tier.primary);

    expect(primaryTier?.id).toBe("analyze");
    expect(primaryTier?.entries[0]).toMatchObject({
      library: "text-analysis",
      presetId: "document-deterministic",
    });
  });
});

function allEntries() {
  return textFamilyTiers.flatMap((tier) => tier.entries);
}

function extractIds(source: string): string[] {
  return Array.from(source.matchAll(/\bid:\s*["']([^"']+)["']/g), (match) => match[1]);
}
