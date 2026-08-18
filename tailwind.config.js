/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        retro: {
          bg: "#dcd6cd",
          window: "#fbf9f5",
          border: "#1a1a1a",
          darkBorder: "#4a4a4a",
          highlight: "#ffffff",
          shadow: "#808080",
          accent: "#2b4c7e",
          titlebar: "#000000",
        },
      },
      fontFamily: {
        mono: ['"Courier New"', 'Courier', 'monospace'],
        retro: ['"Chicago"', '"Geneva"', '"Lucida Console"', 'monospace'],
      },
      imageRendering: {
        pixelated: 'pixelated',
      },
    },
  },
  plugins: [],
};
