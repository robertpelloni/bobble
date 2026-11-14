// Tailwind CSS v4: the PostCSS plugin moved to @tailwindcss/postcss
// (tailwindcss v3 style `tailwindcss: {}` here broke `npm run build`).
export default {
  plugins: {
    '@tailwindcss/postcss': {},
    autoprefixer: {},
  },
}
