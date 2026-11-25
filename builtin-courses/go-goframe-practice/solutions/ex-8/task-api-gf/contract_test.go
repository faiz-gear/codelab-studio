package main

import (
	"io"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/gogf/gf/v2/net/ghttp"
)

func TestGoFrameTaskHTTPContract(t *testing.T) {
	server := ghttp.GetServer(t.Name())
	configure(server)
	server.SetAddr("127.0.0.1:0")
	server.SetDumpRouterMap(false)
	if err := server.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := server.Shutdown(); err != nil {
			t.Error(err)
		}
	})
	base := "http://" + server.GetListenedAddress()
	client := &http.Client{Timeout: 3 * time.Second}

	for _, step := range []struct {
		method, path, body string
		status             int
		output             string
	}{
		{"GET", "/tasks", "", 200, "[]"},
		{"POST", "/tasks", `{"title":" GoFrame "}`, 201, `{"id":1,"title":"GoFrame","status":"pending"}`},
		{"POST", "/tasks", `{"title":"first"}{"title":"second"}`, 400, `{"error":"expected one JSON object containing title"}`},
		{"POST", "/tasks", `{"title":"first","typo":true}`, 400, `{"error":"expected one JSON object containing title"}`},
		{"POST", "/tasks", `{"title":"` + strings.Repeat("x", 16<<10) + `"}`, 413, `{"error":"request body exceeds 16 KiB"}`},
		{"GET", "/tasks/0", "", 400, `{"error":"id must be a positive integer"}`},
		{"GET", "/tasks/1", "", 200, `{"id":1,"title":"GoFrame","status":"pending"}`},
		{"PATCH", "/tasks/1", `{"title":"ORM"}`, 200, `{"id":1,"title":"ORM","status":"pending"}`},
		{"POST", "/tasks/1/complete", "", 200, `{"id":1,"title":"ORM","status":"done"}`},
		{"POST", "/tasks/1/complete", "", 200, `{"id":1,"title":"ORM","status":"done"}`},
		{"DELETE", "/tasks/1", "", 204, ""},
		{"GET", "/tasks/1", "", 404, `{"error":"task not found"}`},
	} {
		request, err := http.NewRequest(step.method, base+step.path, strings.NewReader(step.body))
		if err != nil {
			t.Fatal(err)
		}
		request.Header.Set("Content-Type", "application/json")
		response, err := client.Do(request)
		if err != nil {
			t.Fatal(err)
		}
		body, err := io.ReadAll(response.Body)
		response.Body.Close()
		if err != nil || response.StatusCode != step.status || strings.TrimSpace(string(body)) != step.output {
			t.Fatalf("%s %s: got %d %q (%v), want %d %q", step.method, step.path, response.StatusCode, body, err, step.status, step.output)
		}
	}
}
