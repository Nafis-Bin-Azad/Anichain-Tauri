/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: "#007AFF",
        secondary: "#00b894",
        danger: "#ff3b30",
        warning: "#ff9500",
        success: "#34c759",
        background: "#f5f5f7",
        surface: "#ffffff",
        text: {
          primary: "#333333",
          secondary: "#666666",
        },
      },
    },
  },
  plugins: [],
};
