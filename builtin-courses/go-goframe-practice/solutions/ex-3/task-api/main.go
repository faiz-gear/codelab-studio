package main

import (
	"log"
	"net/http"
	"time"

	"example.com/task-api/internal/httpapi"
	"example.com/task-api/internal/task"
)

func routes() *http.ServeMux {
	return httpapi.Routes(task.NewService(task.NewMemory()))
}

func main() {
	server := &http.Server{
		Addr:              "127.0.0.1:8080",
		Handler:           routes(),
		ReadHeaderTimeout: 5 * time.Second,
	}
	log.Printf("listening on http://%s", server.Addr)
	log.Fatal(server.ListenAndServe())
}
