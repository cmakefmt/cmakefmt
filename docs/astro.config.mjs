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
    sitemap(),
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
      description: "A blazing-fast, workflow-first CMake formatter — built in Rust, built to last.",
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
        {
          tag: "meta",
          attrs: {
            property: "og:image",
            content: "https://cmakefmt.dev/logo.png",
          },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:image:alt",
            content: "cmakefmt logo",
          },
        },
        {
          tag: "meta",
          attrs: {
            name: "twitter:image",
            content: "https://cmakefmt.dev/logo.png",
          },
        },
      ],
      components: {
        ThemeSelect: "./src/components/ThemeToggle.astro",
        ThemeProvider: "./src/components/ThemeProvider.astro",
        Header: "./src/components/Header.astro",
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
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Migration from cmake-format", slug: "migration" },
            { label: "Performance", slug: "performance" },
            { label: "Troubleshooting", slug: "troubleshooting" },
          ],
        },
        {
          label: "Development",
          items: [
            { label: "Library API", slug: "api" },
            { label: "Architecture", slug: "architecture" },
            { label: "Changelog", slug: "changelog" },
          ],
        },
        { label: "Playground", slug: "playground" },
      ],
    }),
  ],
});
