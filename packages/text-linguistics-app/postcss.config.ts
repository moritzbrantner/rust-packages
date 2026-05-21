import autoprefixer from "../../node_modules/.bun/node_modules/autoprefixer/lib/autoprefixer.js";
import tailwindcss from "../../node_modules/.bun/tailwindcss@3.4.19/node_modules/tailwindcss/lib/index.js";

export default {
  plugins: [tailwindcss(), autoprefixer()],
};
