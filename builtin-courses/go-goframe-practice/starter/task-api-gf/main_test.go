package main

import (
	"encoding/json"
	"net/http"
	"testing"
	"time"

	"github.com/gogf/gf/v2/net/ghttp"
)

func TestEmptyTaskListContract(t *testing.T) {
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

	client := &http.Client{Timeout: 3 * time.Second}
	response, err := client.Get("http://" + server.GetListenedAddress() + "/tasks")
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		t.Fatalf("status = %d, want 200", response.StatusCode)
	}
	var tasks []json.RawMessage
	if err := json.NewDecoder(response.Body).Decode(&tasks); err != nil {
		t.Fatal(err)
	}
	if tasks == nil || len(tasks) != 0 {
		t.Fatalf("tasks = %v, want non-null empty list", tasks)
	}
}
