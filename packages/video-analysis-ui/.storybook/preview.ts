import type { Preview } from "@storybook/react-vite";

const preview: Preview = {
  parameters: {
    backgrounds: {
      default: "workspace",
      values: [
        { name: "workspace", value: "#f4f4f5" },
        { name: "surface", value: "#ffffff" },
      ],
    },
    viewport: {
      defaultViewport: "responsive",
    },
    layout: "fullscreen",
  },
};

export default preview;
