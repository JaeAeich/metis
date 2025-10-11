# Metis

<div align="center">
  <img src="./docs/public/web-app-manifest-512x512.png" alt="Metis Logo" width="300"/>
  <br/>
<em align="center">
    A highly-pluggable, GA4GH WES 1.1.0 compliant, Workflow Execution Service.
</em>
</div>
<br/>

Metis is a modern, flexible service for running workflows on Kubernetes. It
brings the power of cloud-native computing to scientific and data-intensive
workflows, acting as a smart manager for your tasks. Built to support
collaboration across different groups, it is compliant with the GA4GH [WES](wes)
standard, ensuring interoperability, and is highly extensible through its
pluggable design.

> [!WARNING]
> Under development

## Design Philosophy

Metis is built around a **core engine** that manages process
lifecycle, validation, and execution orchestration. The engine defines a
`trait-based` contract that workflow runners (Nextflow, Snakemake, CWL, etc.)
implement, enabling seamless pluggability without modifying core logic.

Configuration is externalized via `engine.yaml`, allowing operators to define
command templates, parameter validation rules, and runtime behavior without
code changes. The API layer is fully decoupled from execution, communicating
through NATS/Valkey for async job submission and state tracking.

This separation ensures:

1. **Extensibility**: New engines implement a single trait
2. **Configurability**: Validation and CLI building are template-driven
3. **Resilience**: API and execution layers scale independently

## Versioning

The project adopts the [semantic versioning][semver] scheme for versioning.
Currently the software is in a pre-release stage, so changes to the API,
including breaking changes, may occur at any time without further notice.

## License

This project is distributed under the [Apache License 2.0][badge-license-url], a
copy of which is also available in [`LICENSE`][license].

[badge-license-url]: http://www.apache.org/licenses/LICENSE-2.0
[license]: LICENSE
[semver]: https://semver.org/
