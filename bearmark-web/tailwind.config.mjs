/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class", '[data-theme="night"]'],
  mode: "jit",
  content: {
    files: ["src/**/*.rs", "index.html"],
    extract: {
      rs: (content) => {
        let normal = content.match(/[^<>"'`=\s]*/g);
        let leptos = [...content.matchAll(/:([^<>"'`:=\s]*)=/g)].flat();
        return normal.concat(leptos);
      },
    },
  },
  theme: {},
};