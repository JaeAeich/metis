// Package api provides the API server for Metis.
package api

import (
	"fmt"
	"log"

	"github.com/gofiber/contrib/swagger"
	"github.com/gofiber/fiber/v2"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/api/handlers"
	"github.com/jaeaeich/metis/internal/api/spec"
	"github.com/jaeaeich/metis/internal/config"
)

// Start starts the API server.
func Start() {
	fiberConfig := &fiber.Config{}
	app := fiber.New(*fiberConfig)

	// Health check
	app.Get("/healthz", func(c *fiber.Ctx) error {
		return c.SendStatus(fiber.StatusOK)
	})

	// Swagger UI
	swaggerCfg := swagger.Config{
		BasePath:    config.Cfg.API.Server.BasePath,
		FileContent: spec.Spec,
		Path:        config.Cfg.API.Swagger.Path,
		Title:       config.Cfg.API.Swagger.Title,
	}
	app.Use(swagger.New(swaggerCfg))

	metis := &handlers.Metis{}
	api.RegisterHandlers(app, metis)

	err := app.Listen(fmt.Sprintf(":%d", config.Cfg.API.Server.Port))
	if err != nil {
		log.Fatalf("failed to start server: %v", err)
	}
}
