#!/usr/bin/env nextflow

nextflow.enable.dsl=2

process hello {
    output:
    stdout

    script:
    """
    echo "Hello World from Nextflow!"
    """
}

workflow {
    hello()
}
