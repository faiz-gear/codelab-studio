package main

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
	"unicode/utf8"
)

const (
	Pending = "pending"
	Done    = "done"
)

var (
	ErrNotFound = errors.New("task not found")
	ErrTitle    = errors.New("title must contain 1–100 Unicode code points")
)

type Task struct {
	ID     int64  `json:"id"`
	Title  string `json:"title"`
	Status string `json:"status"`
}

type Service struct {
	*Memory
}

func NewService(store *Memory) *Service {
	return &Service{Memory: store}
}

func (s *Service) Create(ctx context.Context, title string) (Task, error) {
	title, err := normalizeTitle(title)
	if err != nil {
		return Task{}, err
	}
	return s.Memory.Create(ctx, title)
}

func (s *Service) Rename(ctx context.Context, id int64, title string) (Task, error) {
	title, err := normalizeTitle(title)
	if err != nil {
		return Task{}, err
	}
	return s.Memory.Rename(ctx, id, title)
}

func normalizeTitle(title string) (string, error) {
	title = strings.TrimSpace(title)
	if size := utf8.RuneCountInString(title); size < 1 || size > 100 {
		return "", ErrTitle
	}
	return title, nil
}

type Memory struct {
	mu     sync.RWMutex
	nextID int64
	tasks  map[int64]Task
}

func NewMemory() *Memory {
	return &Memory{tasks: make(map[int64]Task)}
}

func (m *Memory) Create(_ context.Context, title string) (Task, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.nextID++
	task := Task{ID: m.nextID, Title: title, Status: Pending}
	m.tasks[task.ID] = task
	return task, nil
}

func (m *Memory) Get(_ context.Context, id int64) (Task, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	task, ok := m.tasks[id]
	if !ok {
		return Task{}, ErrNotFound
	}
	return task, nil
}

func (m *Memory) List(_ context.Context) ([]Task, error) {
	m.mu.RLock()
	tasks := make([]Task, 0, len(m.tasks))
	for _, task := range m.tasks {
		tasks = append(tasks, task)
	}
	m.mu.RUnlock()
	sort.Slice(tasks, func(i, j int) bool { return tasks[i].ID < tasks[j].ID })
	return tasks, nil
}

func (m *Memory) Rename(_ context.Context, id int64, title string) (Task, error) {
	return m.update(id, func(task *Task) { task.Title = title })
}

func (m *Memory) Complete(_ context.Context, id int64) (Task, error) {
	return m.update(id, func(task *Task) { task.Status = Done })
}

func (m *Memory) update(id int64, change func(*Task)) (Task, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	task, ok := m.tasks[id]
	if !ok {
		return Task{}, ErrNotFound
	}
	change(&task)
	m.tasks[id] = task
	return task, nil
}

func (m *Memory) Delete(_ context.Context, id int64) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if _, ok := m.tasks[id]; !ok {
		return ErrNotFound
	}
	delete(m.tasks, id)
	return nil
}

func taskRoutes(service *Service) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.HandleFunc("GET /tasks", func(w http.ResponseWriter, r *http.Request) {
		tasks, err := service.List(r.Context())
		respond(w, http.StatusOK, tasks, err)
	})
	mux.HandleFunc("POST /tasks", func(w http.ResponseWriter, r *http.Request) {
		title, ok := readTitle(w, r)
		if ok {
			item, err := service.Create(r.Context(), title)
			respond(w, http.StatusCreated, item, err)
		}
	})
	mux.HandleFunc("GET /tasks/{id}", withID(func(w http.ResponseWriter, r *http.Request, id int64) {
		item, err := service.Get(r.Context(), id)
		respond(w, http.StatusOK, item, err)
	}))
	mux.HandleFunc("PATCH /tasks/{id}", withID(func(w http.ResponseWriter, r *http.Request, id int64) {
		title, ok := readTitle(w, r)
		if ok {
			item, err := service.Rename(r.Context(), id, title)
			respond(w, http.StatusOK, item, err)
		}
	}))
	mux.HandleFunc("POST /tasks/{id}/complete", withID(func(w http.ResponseWriter, r *http.Request, id int64) {
		item, err := service.Complete(r.Context(), id)
		respond(w, http.StatusOK, item, err)
	}))
	mux.HandleFunc("DELETE /tasks/{id}", withID(func(w http.ResponseWriter, r *http.Request, id int64) {
		respond(w, http.StatusNoContent, nil, service.Delete(r.Context(), id))
	}))
	return mux
}

func withID(handle func(http.ResponseWriter, *http.Request, int64)) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id, err := strconv.ParseInt(r.PathValue("id"), 10, 64)
		if err != nil || id <= 0 {
			writeError(w, http.StatusBadRequest, "id must be a positive integer")
			return
		}
		handle(w, r, id)
	}
}

func readTitle(w http.ResponseWriter, r *http.Request) (string, bool) {
	var body struct {
		Title string `json:"title"`
	}
	decoder := json.NewDecoder(http.MaxBytesReader(w, r.Body, 16<<10))
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
		writeError(w, http.StatusRequestEntityTooLarge, "request body exceeds 16 KiB")
	} else {
		writeError(w, http.StatusBadRequest, "expected one JSON object containing title")
	}
	return "", false
}

func respond(w http.ResponseWriter, status int, value any, err error) {
	switch {
	case errors.Is(err, ErrTitle):
		writeError(w, http.StatusBadRequest, err.Error())
	case errors.Is(err, ErrNotFound):
		writeError(w, http.StatusNotFound, err.Error())
	case err != nil:
		log.Printf("task operation: %v", err)
		writeError(w, http.StatusInternalServerError, "internal server error")
	case status == http.StatusNoContent:
		w.WriteHeader(status)
	default:
		writeJSON(w, status, value)
	}
}

func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(value); err != nil {
		log.Printf("write response: %v", err)
	}
}

func routes() *http.ServeMux {
	return taskRoutes(NewService(NewMemory()))
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
