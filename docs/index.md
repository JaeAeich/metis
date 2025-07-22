---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
    name: "Metis"
    text: "Kubernetes-native Workflow Execution Service"
    tagline: A federation-promoting, highly-pluggable, GA4GH WES 1.1.0 compliant workflow execution service
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
      title: Cloud-Native
      details: Built for Kubernetes, bringing the power of cloud-native computing to scientific and data-intensive workflows
    - icon: 🔌
      title: Pluggable Multi-Engine Support
      details: Extensible architecture supporting multiple workflow engines like Nextflow, Snakemake, CWL, and WDL through a plugin system
    - icon: 🌐
      title: GA4GH WES Compliant
      details: Fully compliant with GA4GH WES 1.1.0 standard, ensuring interoperability and federation across platforms
---
