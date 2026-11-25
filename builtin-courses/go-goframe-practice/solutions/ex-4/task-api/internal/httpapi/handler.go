package httpapi

import (
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"
	"strconv"

	"example.com/task-api/internal/task"
)

func Routes(service *task.Service) *http.ServeMux {
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
	case errors.Is(err, task.ErrTitle):
		writeError(w, http.StatusBadRequest, err.Error())
	case errors.Is(err, task.ErrNotFound):
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
