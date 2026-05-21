import autoprefixer from "../../node_modules/.bun/autoprefixer@10.5.0+4ad1ba81c4bee790/node_modules/autoprefixer";
import tailwindcss from "../../node_modules/.bun/tailwindcss@3.4.19/node_modules/tailwindcss";

export default {
  plugins: [tailwindcss(), autoprefixer()],
};
