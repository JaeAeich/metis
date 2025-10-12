---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: Metis
  text: Kubernetes-native Workflow Execution Service
  tagline: Async-first, trait-based plugins, template-driven configuration
  image:
    src: /web-app-manifest-512x512.png
    alt: Metis Logo
  actions:
    - theme: brand
      text: Get Started
      link: /intro
    - theme: alt
      text: View on GitHub
      link: https://github.com/jaeaeich/metis

features:
  - icon: ⚡
    title: Fast, Practical Runs
    details: Submit workflows quickly and track status without extra ceremony.
  - icon: 🔌
    title: Plug In Any Engine
    details: Add Nextflow, Snakemake, CWL, or WDL support with a single engine trait.
  - icon: ⚙️
    title: Configure, Do Not Fork
    details: Tune validation and command behavior in `engine.yaml` without rewriting core logic.
  - icon: 🌐
    title: Standards Built In
    details: GA4GH WES 1.1.0 compatible for cleaner interoperability across platforms.
---
