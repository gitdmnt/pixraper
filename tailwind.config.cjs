module.exports = {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  theme: {
    extend: {
      colors: {
        "md-surface": "var(--md-surface)",
        "md-on-surface": "var(--md-on-surface)",
        "md-primary": "var(--md-primary)",
        "md-on-primary": "var(--md-on-primary)",
        "md-surface-variant": "var(--md-surface-variant)",
        "md-outline": "var(--md-outline)",
      },
      boxShadow: {
        "md-elev-1":
          "0 1px 2px rgba(16,24,40,0.06), 0 1px 3px rgba(16,24,40,0.04)",
        "md-elev-2":
          "0 4px 8px rgba(16,24,40,0.08), 0 2px 6px rgba(16,24,40,0.06)",
      },
      borderRadius: {
        md: "12px",
      },
    },
  },
  plugins: [],
};
