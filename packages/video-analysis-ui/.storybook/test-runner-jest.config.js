import { getJestConfig } from "@storybook/test-runner";

const testRunnerConfig = getJestConfig();

export default {
  ...testRunnerConfig,
  modulePathIgnorePatterns: [
    ...(testRunnerConfig.modulePathIgnorePatterns ?? []),
    "<rootDir>/../../.external-test-tools/",
    "<rootDir>/../../.audio-tools/",
    "<rootDir>/../../.model-runtime/",
    "<rootDir>/../../.test-corpora/",
  ],
};
