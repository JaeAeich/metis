package main

import (
	"context"
	"fmt"
	"os"

	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("expected 'api','metel', or 'healthz' subcommands")
		os.Exit(1)
	}

	switch os.Args[1] {
	case "api":
		if err := config.LoadAPIConfig(); err != nil {
			fmt.Printf("failed to load API configuration: %v", err)
			os.Exit(1)
		}
		logger.L = logger.New(config.Cfg.Log.Level, config.Cfg.Log.Format)
		if err := initClients(); err != nil {
			logger.L.Error("failed to initialize clients", "error", err)
			os.Exit(1)
		}
		handleAPICmd()
	case "metel":
		if err := config.LoadMetelConfig(); err != nil {
			fmt.Printf("failed to load Metel configuration: %v", err)
			os.Exit(1)
		}
		logger.L = logger.New(config.Cfg.Log.Level, config.Cfg.Log.Format)
		if err := initClients(); err != nil {
			logger.L.Error("failed to initialize clients", "error", err)
			os.Exit(1)
		}
		handleMetelCmd()
	case "healthz":
		handleHealthzCmd()
	default:
		fmt.Println("expected 'api' or 'metel' subcommands")
		os.Exit(1)
	}
}

func initClients() error {
	var err error
	clients.K8s, err = clients.NewK8sClient()
	if err != nil {
		return err
	}
	clients.DB, err = clients.NewMongoClient(context.Background())
	if err != nil {
		return err
	}
	return nil
}

func handleHealthzCmd() {
	fmt.Println("ok")
	os.Exit(0)
}
