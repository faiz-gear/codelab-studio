package main

import (
	"context"
	_ "embed"
	"log"
	"net/http"
	"time"

	"example.com/task-api/internal/httpapi"
	"example.com/task-api/internal/task"
)

//go:embed migrations/001_create_tasks.sql
var createTasks string

func routes() *http.ServeMux {
	return httpapi.Routes(task.NewService(task.NewMemory()))
}

func run() error {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	store, err := task.OpenSQLite(ctx, "tasks.db", createTasks)
	if err != nil {
		return err
	}
	defer store.Close()

	server := &http.Server{
		Addr:              "127.0.0.1:8080",
		Handler:           httpapi.Routes(task.NewService(store)),
		ReadHeaderTimeout: 5 * time.Second,
	}
	log.Printf("listening on http://%s", server.Addr)
	return server.ListenAndServe()
}

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}
