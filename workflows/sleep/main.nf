#!/usr/bin/env nextflow

process sleep_task {

    output:
    stdout

    script:
    """
    echo "Sleeping for 5 seconds..."
    sleep 5
    echo "Done sleeping"
    """
}

workflow {
    sleep_task()
}
