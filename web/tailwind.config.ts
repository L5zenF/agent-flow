import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        paper: "#efe7d7",
        ink: "#151515",
        steel: "#3f3f46",
        ember: "#c65d16",
        sand: "#d9cdb7",
        moss: "#5b6b4d",
      },
      fontFamily: {
        sans: ["'IBM Plex Sans'", "sans-serif"],
        mono: ["'IBM Plex Mono'", "monospace"],
      },
      boxShadow: {
        panel: "0 20px 60px rgba(21, 21, 21, 0.16)",
      },
    },
  },
  plugins: [],
} satisfies Config;
