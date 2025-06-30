package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("expected 'api' or 'metel' subcommands")
		os.Exit(1)
	}

	switch os.Args[1] {
	case "api":
		handleApiCmd()
	case "metel":
		handleMetelCmd()
	default:
		fmt.Println("expected 'api' or 'metel' subcommands")
		os.Exit(1)
	}
}
