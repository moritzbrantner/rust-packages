type StorybookEntry = {
  id?: unknown;
  type?: unknown;
};

export function storyIdsFromIndex(index: unknown): string[] {
  const entries =
    index && typeof index === "object"
      ? (index as { entries?: unknown }).entries
      : undefined;
  const storyIds =
    entries && typeof entries === "object"
      ? Object.values(entries)
          .filter(
            (entry): entry is StorybookEntry =>
              Boolean(entry) &&
              typeof entry === "object" &&
              (entry as StorybookEntry).type === "story" &&
              typeof (entry as StorybookEntry).id === "string",
          )
          .map((entry) => entry.id as string)
      : [];

  if (storyIds.length === 0) {
    throw new Error("Storybook index did not contain any stories.");
  }
  return storyIds;
}
