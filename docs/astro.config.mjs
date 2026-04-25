// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// @ts-check
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://cmakefmt.dev/",
  vite: {
    plugins: [tailwindcss()],
  },
  markdown: {
    shikiConfig: {
      langs: ["cmake", "rust", "bash", "yaml", "toml", "diff", "text", "python"],
    },
  },
  integrations: [
    sitemap({
      // Apply build time as <lastmod> on every URL so crawlers know
      // the snapshot's age. Pages that haven't changed structurally
      // still get a fresh timestamp on each deploy — that's fine; it
      // just nudges crawlers to revisit, which is what we want.
      lastmod: new Date(),
    }),
    starlight({
      expressiveCode: {
        themes: ["github-dark-default", "github-light-default"],
        styleOverrides: {
          borderRadius: "1rem",
          borderWidth: "0px",
          borderColor: "transparent",
          codeFontSize: "0.8125rem",
          codePaddingBlock: "1rem",
          codePaddingInline: "1.25rem",
          frames: {
            frameBoxShadowCssValue: "0 0 0 1px var(--sl-color-gray-5)",
            editorActiveTabIndicatorTopColor: "var(--sl-color-accent)",
            editorActiveTabIndicatorBottomColor: "transparent",
          },
        },
      },
      title: "cmakefmt",
      description: "A lightning-fast, workflow-first CMake formatter — built in Rust, built to last.",
      logo: {
        src: "./src/assets/logo.png",
        alt: "cmakefmt logo",
      },
      favicon: "/favicon.png",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/cmakefmt/cmakefmt",
        },
      ],
      head: [
        // Modern favicon link alongside Starlight's built-in
        // `<link rel="shortcut icon">` (which is the legacy form).
        // Helps browsers like Safari refetch when the icon changes.
        {
          tag: "link",
          attrs: {
            rel: "icon",
            type: "image/png",
            sizes: "32x32",
            href: "/favicon.png",
          },
        },
        // Apple touch icon for iOS Safari "Add to Home Screen" and
        // macOS Safari Reading List previews. Without this, Safari
        // probes for /apple-touch-icon.png and falls back to a
        // generic if the probe 404s, which can leak through as a
        // mismatched icon.
        {
          tag: "link",
          attrs: {
            rel: "apple-touch-icon",
            sizes: "180x180",
            href: "/apple-touch-icon.png",
          },
        },
        {
          tag: "meta",
          attrs: { property: "og:type", content: "website" },
        },
        {
          tag: "meta",
          attrs: { property: "og:site_name", content: "cmakefmt" },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:image",
            content: "https://cmakefmt.dev/og-image.png",
          },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:width", content: "1726" },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:height", content: "911" },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:image:alt",
            content: "cmakefmt — cmake-format, reimagined",
          },
        },
      ],
      components: {
        ThemeSelect: "./src/components/ThemeToggle.astro",
        Header: "./src/components/Header.astro",
        MobileMenuToggle: "./src/components/MobileMenuToggle.astro",
        MobileTableOfContents: "./src/components/MobileTableOfContents.astro",
        Sidebar: "./src/components/Sidebar.astro",
        Footer: "./src/components/Footer.astro",
      },
      lastUpdated: true,
      customCss: [
        // Tailwind v4 entry point — base styles sit in @layer base, so
        // Starlight's component styles naturally take precedence.
        "./src/styles/starwind.css",
        // Brand colours and layout tweaks.
        "./src/styles/custom.css",
      ],
      sidebar: [
        {
          label: "Getting Started",
          items: [
            { label: "Getting Started", slug: "getting-started" },
            { label: "Installation", slug: "installation" },
            { label: "Coverage", slug: "coverage" },
            { label: "Release Channels", slug: "release" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI Reference", slug: "cli" },
            { label: "Config Reference", slug: "config" },
            { label: "Formatter Behavior", slug: "behavior" },
            { label: "Glossary", slug: "glossary" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Formatting Cookbook", slug: "cookbook" },
            { label: "Migration from cmake-format", slug: "migration" },
            { label: "Editor Integration", slug: "editors" },
            { label: "CI Integration", slug: "ci" },
            { label: "Performance", slug: "performance" },
            { label: "Comparison", slug: "comparison" },
            { label: "Troubleshooting", slug: "troubleshooting" },
          ],
        },
        {
          label: "Community",
          items: [
            { label: "Badge", slug: "badge" },
            { label: "Projects using cmakefmt", slug: "users" },
          ],
        },
        {
          label: "Development",
          items: [
            { label: "Library API", slug: "api" },
            { label: "WASM API", slug: "wasm-api" },
            { label: "Architecture", slug: "architecture" },
            { label: "Stability Contract", slug: "stability" },
            { label: "Contributing", slug: "contributing" },
            { label: "Changelog", slug: "changelog" },
          ],
        },
        { label: "Playground", slug: "playground" },
      ],
    }),
  ],
});
