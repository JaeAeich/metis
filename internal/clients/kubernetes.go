// Package clients provides the kubernetes client for Metis.
package clients

import (
	"log"

	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/tools/clientcmd"

	"github.com/jaeaeich/metis/internal/config"
)

// K8s is the global kubernetes client.
var K8s *kubernetes.Clientset

// NewK8sClient creates a new kubernetes client.
func NewK8sClient() (*kubernetes.Clientset, error) {
	// If the config path is not provided, use the in-cluster config.
	if config.Cfg.K8s.ConfigPath == "" {
		log.Println("using in-cluster config")
		config, err := clientcmd.BuildConfigFromFlags("", "")
		if err != nil {
			return nil, err
		}
		return kubernetes.NewForConfig(config)
	}
	log.Println("using kubeconfig from path")
	config, err := clientcmd.BuildConfigFromFlags("", config.Cfg.K8s.ConfigPath)
	if err != nil {
		return nil, err
	}
	return kubernetes.NewForConfig(config)
}
