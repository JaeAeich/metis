package main

import (
	"flag"
	"fmt"
	"os"
)

func handleMetelCmd() {
	metelCmd := flag.NewFlagSet("metel", flag.ExitOnError)
	workflow := metelCmd.String("workflow", "", "workflow to run")

	metelCmd.Parse(os.Args[2:])

	if *workflow == "" {
		fmt.Println("please provide a workflow")
		os.Exit(1)
	}

	fmt.Printf("running workflow: %s\n", *workflow)
}
