package main

import (
	"fmt"
	"log"
	"os"

	"github.com/jaeaeich/metis/internal/config"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("expected 'api' or 'metel' subcommands")
		os.Exit(1)
	}

	switch os.Args[1] {
	case "api":
		if err := config.LoadAPIConfig(); err != nil {
			log.Fatalf("failed to load API configuration: %v", err)
		}
		handleAPICmd()
	case "metel":
		if err := config.LoadMetelConfig(); err != nil {
			log.Fatalf("failed to load Metel configuration: %v", err)
		}
		handleMetelCmd()
	default:
		fmt.Println("expected 'api' or 'metel' subcommands")
		os.Exit(1)
	}
}
