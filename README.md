# Metis

<div align="center">
  <img src="./docs/public/web-app-manifest-512x512.png" alt="Metis Logo" width="300"/>
  <br/>
<em align="center">A Kubernetes-native, federation-promoting, highly-pluggable,
GA4GH WES 1.1.0 compliant workflow execution service.</em>
</div>
<br/>

Metis is a modern, flexible service for running workflows on Kubernetes. It
brings the power of cloud-native computing to scientific and data-intensive
workflows, acting as a smart manager for your tasks. Built to support
collaboration across different groups, it is compliant with the GA4GH [WES](wes)
standard, ensuring interoperability, and is highly extensible through its
pluggable design.

## Table of Contents

- [Basic Usage](#basic-usage)
- [Installation](#installation)
- [Development](#development)
  - [Makefile](#makefile)
  - [Environment reproducibility](#environment-reproducibility)
    - [Editor config](#editor-config)
    - [Setting environment variables (direnv)](#setting-environment-variables-direnv)
- [Versioning](#versioning)
- [License](#license)

## Basic Usage

## Installation

## Development

### Makefile

For ease of use, certain scripts have been abbreviated in `Makefile`, make sure
that you have installed the dependencies before running the commands.

### Environment reproducibility

#### Editor Config

To ensure a consistent code style across the project, we include an
`.editorconfig` file that defines the coding styles for different editors and
IDEs. Most modern editors support this file format out of the box, but you might
need to install a plugin for some editors. Please refer to the
[EditorConfig website][editor-config].

#### Setting environment variables (direnv)

Our project uses [.envrc files][direnv] to manage environment variables.
Wherever such a file is required across the project tree, you will find a
`.envrc.template` file that contains the necessary variables and helps you
create your own personal copy of each file. You can find the locations of all
`.envrc.template` files by executing `find . -type f -name \.envrc\.template` in
the root directory. For each, create a copy named `.envrc` in the same
directory, open it in a text editor and replace the template/example values with
your own personal and potentially confidential values.

**Warning:** Be careful not to leak sensitive information! In particular,
**never** add your secrets to the `.envrc.template` files directly, as these are
committed to version control and will be visible to anyone with access to the
repository. Always create an `.envrc` copy first (capitalization and punctuation
matter!), as these (along with `.env` files) are ignored from version control.

Once you have filled in all of your personal information, you can have the
`direnv` tool manage setting your environment variables automatically (depending
on the directory you are currently in and the particular `.envrc` file defined
for that directory) by executing the following command:

```sh
direnv allow
```

## Versioning

The project adopts the [semantic versioning][semver] scheme for versioning.
Currently the software is in a pre-release stage, so changes to the API,
including breaking changes, may occur at any time without further notice.

## License

This project is distributed under the [Apache License 2.0][badge-license-url], a
copy of which is also available in [`LICENSE`][license].

[badge-license-url]: http://www.apache.org/licenses/LICENSE-2.0
[direnv]: https://direnv.net/
[editor-config]: https://editorconfig.org/
[license]: LICENSE
[semver]: https://semver.org/
