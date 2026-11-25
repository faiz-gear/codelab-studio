package main

import (
	"context"
	_ "embed"
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"
	"strconv"
	"time"

	"example.com/task-api-gf/internal/task"
	"github.com/gogf/gf/v2/frame/g"
	"github.com/gogf/gf/v2/net/ghttp"
)

//go:embed migrations/001_create_tasks.sql
var createTasks string

//go:embed migrations/002_create_task_events.sql
var createEvents string

func configure(server *ghttp.Server) {
	bindRoutes(server, task.NewService(task.NewMemory()))
}

func bindRoutes(server *ghttp.Server, service *task.Service) {
	server.SetAddr("127.0.0.1:8000")
	server.BindHandler("GET:/healthz", func(r *ghttp.Request) {
		respond(r, http.StatusOK, map[string]string{"status": "ok"}, nil)
	})
	server.BindHandler("GET:/tasks", func(r *ghttp.Request) {
		tasks, err := service.List(r.Context())
		respond(r, http.StatusOK, tasks, err)
	})
	server.BindHandler("POST:/tasks", func(r *ghttp.Request) {
		title, ok := readTitle(r)
		if ok {
			item, err := service.Create(r.Context(), title)
			respond(r, http.StatusCreated, item, err)
		}
	})
	server.BindHandler("GET:/tasks/{id}", withID(func(r *ghttp.Request, id int64) {
		item, err := service.Get(r.Context(), id)
		respond(r, http.StatusOK, item, err)
	}))
	server.BindHandler("PATCH:/tasks/{id}", withID(func(r *ghttp.Request, id int64) {
		title, ok := readTitle(r)
		if ok {
			item, err := service.Rename(r.Context(), id, title)
			respond(r, http.StatusOK, item, err)
		}
	}))
	server.BindHandler("POST:/tasks/{id}/complete", withID(func(r *ghttp.Request, id int64) {
		item, err := service.Complete(r.Context(), id)
		respond(r, http.StatusOK, item, err)
	}))
	server.BindHandler("DELETE:/tasks/{id}", withID(func(r *ghttp.Request, id int64) {
		// GoFrame supplies the routing and response writer. The business layer
		// still defines the title and status rules for every caller.
		respond(r, http.StatusNoContent, nil, service.Delete(r.Context(), id))
	}))
}

func withID(handle func(*ghttp.Request, int64)) ghttp.HandlerFunc {
	return func(r *ghttp.Request) {
		id, err := strconv.ParseInt(r.GetRouter("id", "").String(), 10, 64)
		if err != nil || id <= 0 {
			writeError(r, http.StatusBadRequest, "id must be a positive integer")
			return
		}
		handle(r, id)
	}
}

func readTitle(r *ghttp.Request) (string, bool) {
	var body struct {
		Title string `json:"title"`
	}
	// Keep the standard-library decoder so the framework migration preserves
	// strict field validation, single-object input, and the 16 KiB bound.
	decoder := json.NewDecoder(http.MaxBytesReader(r.Response.Writer, r.Body, 16<<10))
	decoder.DisallowUnknownFields()
	err := decoder.Decode(&body)
	if err == nil {
		err = decoder.Decode(new(json.RawMessage))
		if err == io.EOF {
			return body.Title, true
		}
	}
	var tooLarge *http.MaxBytesError
	if errors.As(err, &tooLarge) {
		writeError(r, http.StatusRequestEntityTooLarge, "request body exceeds 16 KiB")
	} else {
		writeError(r, http.StatusBadRequest, "expected one JSON object containing title")
	}
	return "", false
}

func respond(r *ghttp.Request, status int, value any, err error) {
	switch {
	case errors.Is(err, task.ErrTitle):
		writeError(r, http.StatusBadRequest, err.Error())
	case errors.Is(err, task.ErrNotFound):
		writeError(r, http.StatusNotFound, err.Error())
	case err != nil:
		log.Printf("task operation: %v", err)
		writeError(r, http.StatusInternalServerError, "internal server error")
	default:
		r.Response.WriteHeader(status)
		if status != http.StatusNoContent {
			r.Response.WriteJson(value)
		}
	}
}

func writeError(r *ghttp.Request, status int, message string) {
	r.Response.WriteHeader(status)
	r.Response.WriteJson(map[string]string{"error": message})
}

func run() error {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	store, err := task.OpenORM(ctx, "tasks.db", createTasks, createEvents)
	if err != nil {
		return err
	}
	defer store.Close(context.Background())
	server := g.Server()
	bindRoutes(server, task.NewService(store))
	server.Run()
	return nil
}

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}
