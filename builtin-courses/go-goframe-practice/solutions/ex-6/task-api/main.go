package main

import (
	"context"
	_ "embed"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"example.com/task-api/internal/httpapi"
	"example.com/task-api/internal/task"
)

//go:embed migrations/001_create_tasks.sql
var createTasks string

//go:embed migrations/002_create_task_events.sql
var createEvents string

func routes() *http.ServeMux {
	return httpapi.Routes(task.NewService(task.NewMemory()))
}

func run() error {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	store, err := task.OpenSQLite(ctx, "tasks.db", createTasks, createEvents)
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
	signalCtx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	stopped := make(chan error, 1)
	go func() {
		stopped <- server.ListenAndServe()
	}()
	select {
	case err := <-stopped:
		return err
	case <-signalCtx.Done():
		stop()
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := server.Shutdown(shutdownCtx); err != nil {
			_ = server.Close()
			return err
		}
		if err := <-stopped; !errors.Is(err, http.ErrServerClosed) {
			return err
		}
		return nil
	}
}

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}
