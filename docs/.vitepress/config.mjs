import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid";

// https://vitepress.dev/reference/site-config
export default withMermaid(
    defineConfig({
        title: "Metis",
        head: [["link", { rel: "icon", href: "/favicon.ico" }]],
        description:
            "Workflow Execution Service - A federation-promoting, highly-pluggable, GA4GH WES 1.1.0 compliant workflow execution service",
        themeConfig: {
            // https://vitepress.dev/reference/default-theme-config
            logo: "/web-app-manifest-512x512.png",
            nav: [
                { text: "Home", link: "/" },
                { text: "Docs", link: "/intro" },
                { text: "API", link: "/api-reference" },
                { text: "GitHub", link: "https://github.com/jaeaeich/metis" },
            ],
            sidebar: [
                {
                    text: "Getting Started",
                    items: [
                        { text: "Introduction", link: "/intro" },
                        { text: "Quick Start", link: "/getting-started" },
                    ],
                },
                {
                    text: "Guides",
                    items: [
                        { text: "Engine Configuration", link: "/engine-configuration" },
                        { text: "Deployment", link: "/deployment" },
                    ],
                },
                {
                    text: "Reference",
                    items: [
                        { text: "API Reference", link: "/api-reference" },
                        { text: "Architecture", link: "/architecture" },
                    ],
                },
                {
                    text: "Dev",
                    items: [
                        { text: "Engine Internals", link: "/dev/engine" },
                        { text: "Tracing", link: "/dev/tracing" },
                    ],
                },
            ],
            search: {
                provider: "local",
            },
            socialLinks: [
                { icon: "github", link: "https://github.com/jaeaeich/metis" },
            ],
            footer: {
                message: "Released under the Apache License 2.0.",
                copyright: "Copyright © 2025 jaeaeich (Javed Habib)",
            },
            outline: {
                level: "deep",
                label: "On this page",
            },
            editLink: {
                pattern: "https://github.com/jaeaeich/metis/edit/main/docs/:path",
                text: "Edit this page on GitHub",
            },
            docLayout: "doc",
        },
        markdown: {
            theme: {
                light: "github-light",
                dark: "one-dark-pro",
            },
        },
        ignoreDeadLinks: [
            // ignore all localhost links
            /^https?:\/\/localhost/,
            (url) => {
                return url.toLowerCase().includes("ignore");
            },
        ],
    }),
    {
        mermaid: {},
        mermaidPlugin: {
            class: "mermaid",
        },
    }
);
