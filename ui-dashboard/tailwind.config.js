import typography from '@tailwindcss/typography';

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        dcops: {
          50: '#f0f4f6',
          100: '#d9e4ea',
          200: '#b8ced9',
          300: '#8fb0c1',
          400: '#5a6c5d',
          500: '#4a5a4c',
          600: '#3d4a3e',
          700: '#333d34',
          800: '#2d342e',
          900: '#1f2528',
        },
      },
    },
  },
  plugins: [
    typography,
  ],
}
