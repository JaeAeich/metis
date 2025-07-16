package clients

import (
	"context"
	"fmt"

	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"

	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
)

// DB is the global mongo client.
var DB *mongo.Client

// NewMongoClient creates a new mongo client.
func NewMongoClient(ctx context.Context) (*mongo.Client, error) {
	opts := options.Client().ApplyURI(fmt.Sprintf("mongodb://%s:%d",
		config.Cfg.Mongo.Host,
		config.Cfg.Mongo.Port,
	)).SetAuth(options.Credential{
		Username: config.Cfg.Mongo.Username,
		Password: config.Cfg.Mongo.Password,
	})
	client, err := mongo.Connect(ctx, opts)
	logger.L.Debug("connecting to mongo", "uri", opts.GetURI())
	if err != nil {
		return nil, err
	}
	return client, nil
}
