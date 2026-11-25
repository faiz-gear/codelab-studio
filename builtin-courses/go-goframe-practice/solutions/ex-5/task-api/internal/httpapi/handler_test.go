package httpapi

import (
	"net/http/httptest"
	"strings"
	"testing"

	"example.com/task-api/internal/task"
)

func TestTaskHTTPContract(t *testing.T) {
	routes := Routes(task.NewService(task.NewMemory()))
	for _, step := range []struct {
		method, path, input string
		status              int
		output              string
	}{
		{"GET", "/tasks", "", 200, "[]"},
		{"POST", "/tasks", `{"title":" 学习 "}`, 201, `{"id":1,"title":"学习","status":"pending"}`},
		{"POST", "/tasks", `{"title":"first"}{"title":"second"}`, 400, `{"error":"expected one JSON object containing title"}`},
		{"POST", "/tasks", `{"title":"first","typo":true}`, 400, `{"error":"expected one JSON object containing title"}`},
		{"POST", "/tasks", `{"title":"` + strings.Repeat("x", 16<<10) + `"}`, 413, `{"error":"request body exceeds 16 KiB"}`},
		{"GET", "/tasks/0", "", 400, `{"error":"id must be a positive integer"}`},
		{"GET", "/tasks/1", "", 200, `{"id":1,"title":"学习","status":"pending"}`},
		{"PATCH", "/tasks/1", `{"title":"练习"}`, 200, `{"id":1,"title":"练习","status":"pending"}`},
		{"POST", "/tasks/1/complete", "", 200, `{"id":1,"title":"练习","status":"done"}`},
		{"POST", "/tasks/1/complete", "", 200, `{"id":1,"title":"练习","status":"done"}`},
		{"DELETE", "/tasks/1", "", 204, ""},
		{"GET", "/tasks/1", "", 404, `{"error":"task not found"}`},
	} {
		response := httptest.NewRecorder()
		routes.ServeHTTP(response, httptest.NewRequest(step.method, step.path, strings.NewReader(step.input)))
		if response.Code != step.status || strings.TrimSpace(response.Body.String()) != step.output {
			t.Fatalf("%s %s: got %d %q, want %d %q", step.method, step.path, response.Code, response.Body, step.status, step.output)
		}
	}
}
